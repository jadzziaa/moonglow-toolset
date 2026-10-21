//! Two passes over glslang's SPIR-V words before naga reads them, as
//! `spirv-opt --split-combined-image-sampler` and glslang's own mapping
//! would do them:
//!
//! - **Combined image samplers split** ([`split_samplers`]). GLSL 330's
//!   `sampler2D` is one object; naga has textures and samplers apart. Each
//!   such variable becomes `NAME_image` and `NAME_sampler`, each function
//!   parameter two parameters (`DisplaceTextureCoords` and
//!   `SetupOcclusionFromTexture` take one), each load a load of both and an
//!   `OpSampledImage`. Arrays of samplers aren't split (no shader seen has
//!   one): such a program is refused.
//! - **Spare blocks** ([`add_spare_blocks`]): two more variables of the
//!   uniform block's type, which [`crate::Program::split_uniforms`] makes
//!   the frame's and the bones' blocks in the IR. (naga gives every
//!   function an expression for each global variable as it reads the
//!   module, before the function's own: declared here, the spare blocks'
//!   come before every access the split takes to them, as naga's handles
//!   must.)
//! - **Locations** ([`assign_locations`]): every input and output without
//!   a `Location` or `BuiltIn` gets the next free one, in the order glslang
//!   declares them. M1 gives vertex inputs Moonglow's own locations and
//!   matches the varyings by name, in the IR.
//!
//! The words come from glslang, but the passes check every index all the
//! same: a module they don't understand is an error, not a panic.

use std::collections::{HashMap, HashSet};

use crate::ShaderError;

const MAGIC: u32 = 0x0723_0203;

const OP_NAME: u16 = 5;
const OP_TYPE_BOOL: u16 = 20;
const OP_TYPE_INT: u16 = 21;
const OP_TYPE_FLOAT: u16 = 22;
const OP_TYPE_VECTOR: u16 = 23;
const OP_TYPE_MATRIX: u16 = 24;
const OP_TYPE_SAMPLER: u16 = 26;
const OP_TYPE_SAMPLED_IMAGE: u16 = 27;
const OP_TYPE_ARRAY: u16 = 28;
const OP_TYPE_STRUCT: u16 = 30;
const OP_TYPE_POINTER: u16 = 32;
const OP_TYPE_FUNCTION: u16 = 33;
const OP_CONSTANT: u16 = 43;
const OP_FUNCTION_PARAMETER: u16 = 55;
const OP_FUNCTION_CALL: u16 = 57;
const OP_VARIABLE: u16 = 59;
const OP_LOAD: u16 = 61;
const OP_DECORATE: u16 = 71;
const OP_MEMBER_DECORATE: u16 = 72;
const OP_SAMPLED_IMAGE: u16 = 86;

const DECORATION_BUILTIN: u32 = 11;
const DECORATION_LOCATION: u32 = 30;
const DECORATION_BINDING: u32 = 33;
const DECORATION_DESCRIPTOR_SET: u32 = 34;

const DECORATION_BLOCK: u32 = 2;

const STORAGE_UNIFORM_CONSTANT: u32 = 0;
const STORAGE_UNIFORM: u32 = 2;
const STORAGE_INPUT: u32 = 1;
const STORAGE_OUTPUT: u32 = 3;

fn bad(what: impl Into<String>) -> ShaderError {
    ShaderError::Spirv(what.into())
}

/// One instruction: its opcode and operands (the words after the first).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Inst {
    op: u16,
    args: Vec<u32>,
}

impl Inst {
    fn new(op: u16, args: Vec<u32>) -> Inst {
        Inst { op, args }
    }

    fn arg(&self, i: usize) -> Result<u32, ShaderError> {
        self.args.get(i).copied().ok_or_else(|| bad(format!("opcode {} is too short", self.op)))
    }

    /// Whether it declares a type, constant or global variable (the module
    /// section after the decorations).
    fn is_declaration(&self) -> bool {
        matches!(self.op, 19..=39 | 41..=52 | OP_VARIABLE | 1 /* OpUndef */)
    }
}

/// A module: its header and instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Module {
    header: [u32; 5],
    insts: Vec<Inst>,
}

impl Module {
    fn parse(words: &[u32]) -> Result<Module, ShaderError> {
        let header: [u32; 5] = words
            .get(..5)
            .and_then(|h| h.try_into().ok())
            .ok_or_else(|| bad("shorter than a SPIR-V header"))?;
        if header[0] != MAGIC {
            return Err(bad("not SPIR-V"));
        }
        let mut insts = Vec::new();
        let mut i = 5;
        while i < words.len() {
            let first = words[i];
            let count = (first >> 16) as usize;
            let end = i.checked_add(count).filter(|&e| count > 0 && e <= words.len());
            let end =
                end.ok_or_else(|| bad(format!("an instruction at word {i} runs past the end")))?;
            insts.push(Inst::new((first & 0xffff) as u16, words[i + 1..end].to_vec()));
            i = end;
        }
        Ok(Module { header, insts })
    }

    fn words(&self) -> Vec<u32> {
        let mut out = self.header.to_vec();
        for inst in &self.insts {
            out.push(((inst.args.len() as u32 + 1) << 16) | u32::from(inst.op));
            out.extend_from_slice(&inst.args);
        }
        out
    }

    fn new_id(&mut self) -> u32 {
        let id = self.header[3];
        self.header[3] += 1;
        id
    }
}

/// A literal string's words (null-terminated, padded to a word).
fn string_words(s: &str) -> Vec<u32> {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    while !bytes.len().is_multiple_of(4) {
        bytes.push(0);
    }
    bytes.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

/// The literal string that starts operand `from`.
fn string_at(args: &[u32], from: usize) -> String {
    let bytes: Vec<u8> = args.iter().skip(from).flat_map(|w| w.to_le_bytes()).collect();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// Splits every combined image sampler into an image and a sampler.
pub(crate) fn split_samplers(words: &[u32]) -> Result<Vec<u32>, ShaderError> {
    let mut m = Module::parse(words)?;
    // Sampled-image types and their images.
    let mut sampled: HashMap<u32, u32> = HashMap::new();
    let mut existing_sampler = None;
    for inst in &m.insts {
        match inst.op {
            OP_TYPE_SAMPLED_IMAGE => {
                sampled.insert(inst.arg(0)?, inst.arg(1)?);
            }
            OP_TYPE_SAMPLER => existing_sampler = Some(inst.arg(0)?),
            _ => {}
        }
    }
    if sampled.is_empty() {
        return Ok(words.to_vec());
    }
    for inst in &m.insts {
        if inst.op == OP_TYPE_ARRAY && sampled.contains_key(&inst.arg(1)?) {
            return Err(bad("an array of samplers (not split yet)"));
        }
    }
    if existing_sampler.is_some() {
        return Err(bad("a separate sampler type beside combined ones"));
    }
    // UniformConstant pointers to them: their new image and sampler pointers.
    let sampler_type = m.new_id();
    let mut pointers: HashMap<u32, (u32, u32, u32)> = HashMap::new(); // ptr -> (image, image ptr, sampler ptr)
    for inst in &m.insts {
        if inst.op == OP_TYPE_POINTER
            && inst.arg(1)? == STORAGE_UNIFORM_CONSTANT
            && let Some(&image) = sampled.get(&inst.arg(2)?)
        {
            pointers.insert(inst.arg(0)?, (image, 0, 0));
        }
    }
    // (In id order: the same words every run.)
    let mut ids: Vec<u32> = pointers.keys().copied().collect();
    ids.sort_unstable();
    for p in ids {
        let (ip, sp) = (m.new_id(), m.new_id());
        if let Some(e) = pointers.get_mut(&p) {
            e.1 = ip;
            e.2 = sp;
        }
    }
    // Variables and parameters of those pointer types: their sampler halves.
    let mut halves: HashMap<u32, u32> = HashMap::new();
    let objects: Vec<u32> = m
        .insts
        .iter()
        .filter(|i| matches!(i.op, OP_VARIABLE | OP_FUNCTION_PARAMETER))
        .filter(|i| i.args.first().is_some_and(|t| pointers.contains_key(t)))
        .map(|i| i.arg(1))
        .collect::<Result<_, _>>()?;
    for o in objects {
        let s = m.new_id();
        halves.insert(o, s);
    }
    let names: HashMap<u32, String> = m
        .insts
        .iter()
        .filter(|i| i.op == OP_NAME && i.args.first().is_some_and(|t| halves.contains_key(t)))
        .map(|i| (i.args[0], string_at(&i.args, 1)))
        .collect();

    let mut out = Vec::with_capacity(m.insts.len() + 4 * halves.len());
    let mut sampler_declared = false;
    for inst in std::mem::take(&mut m.insts) {
        match inst.op {
            OP_NAME if halves.contains_key(&inst.arg(0)?) => {
                let (id, name) = (inst.arg(0)?, &names[&inst.arg(0)?]);
                let mut a = vec![id];
                a.extend(string_words(&format!("{name}_image")));
                out.push(Inst::new(OP_NAME, a));
                let mut a = vec![halves[&id]];
                a.extend(string_words(&format!("{name}_sampler")));
                out.push(Inst::new(OP_NAME, a));
            }
            OP_DECORATE if halves.contains_key(&inst.arg(0)?) => {
                let half = halves[&inst.arg(0)?];
                let d = inst.arg(1)?;
                let copy = (d == DECORATION_BINDING || d == DECORATION_DESCRIPTOR_SET).then(|| {
                    let mut a = inst.args.clone();
                    a[0] = half;
                    Inst::new(OP_DECORATE, a)
                });
                out.push(inst);
                out.extend(copy);
            }
            OP_TYPE_POINTER if pointers.contains_key(&inst.arg(0)?) => {
                let (image, ip, sp) = pointers[&inst.arg(0)?];
                out.push(inst);
                if !sampler_declared {
                    out.push(Inst::new(OP_TYPE_SAMPLER, vec![sampler_type]));
                    sampler_declared = true;
                }
                out.push(Inst::new(OP_TYPE_POINTER, vec![ip, STORAGE_UNIFORM_CONSTANT, image]));
                out.push(Inst::new(
                    OP_TYPE_POINTER,
                    vec![sp, STORAGE_UNIFORM_CONSTANT, sampler_type],
                ));
            }
            OP_TYPE_FUNCTION if inst.args.iter().skip(2).any(|t| pointers.contains_key(t)) => {
                let mut a = inst.args[..2].to_vec();
                for t in &inst.args[2..] {
                    match pointers.get(t) {
                        Some(&(_, ip, sp)) => a.extend([ip, sp]),
                        None => a.push(*t),
                    }
                }
                out.push(Inst::new(OP_TYPE_FUNCTION, a));
            }
            OP_VARIABLE | OP_FUNCTION_PARAMETER if halves.contains_key(&inst.arg(1)?) => {
                let (_, ip, sp) = pointers[&inst.arg(0)?];
                let mut image = inst.args.clone();
                image[0] = ip;
                let mut sampler = inst.args.clone();
                sampler[0] = sp;
                sampler[1] = halves[&inst.arg(1)?];
                if inst.op == OP_VARIABLE {
                    sampler.truncate(3); // no initializer
                }
                out.push(Inst::new(inst.op, image));
                out.push(Inst::new(inst.op, sampler));
            }
            OP_FUNCTION_CALL if inst.args.iter().skip(3).any(|a| halves.contains_key(a)) => {
                let mut a = inst.args[..3].to_vec();
                for arg in &inst.args[3..] {
                    a.push(*arg);
                    a.extend(halves.get(arg));
                }
                out.push(Inst::new(OP_FUNCTION_CALL, a));
            }
            OP_LOAD if halves.contains_key(&inst.arg(2)?) => {
                let (ty, result, ptr) = (inst.arg(0)?, inst.arg(1)?, inst.arg(2)?);
                let image =
                    *sampled.get(&ty).ok_or_else(|| bad("a sampler loaded as another type"))?;
                let (li, ls) = (m.header[3], m.header[3] + 1);
                m.header[3] += 2;
                out.push(Inst::new(OP_LOAD, vec![image, li, ptr]));
                out.push(Inst::new(OP_LOAD, vec![sampler_type, ls, halves[&ptr]]));
                out.push(Inst::new(OP_SAMPLED_IMAGE, vec![ty, result, li, ls]));
            }
            _ => out.push(inst),
        }
    }
    m.insts = out;
    Ok(m.words())
}

/// The names of the spare blocks [`add_spare_blocks`] declares: the
/// frame's, then the bones'.
pub(crate) const SPARE_BLOCKS: [&str; 2] = ["mg_frame", "mg_bones"];

/// Declares [`SPARE_BLOCKS`] beside the program's uniform block (glslang's
/// `gl_DefaultUniformBlock`, the one `Uniform` variable of a `Block`
/// struct): variables of its type, bound as it is. A program without one
/// block of loose uniforms is left as it is.
pub(crate) fn add_spare_blocks(words: &[u32]) -> Result<Vec<u32>, ShaderError> {
    let mut m = Module::parse(words)?;
    let blocks: HashSet<u32> = m
        .insts
        .iter()
        .filter(|i| i.op == OP_DECORATE && i.args.get(1) == Some(&DECORATION_BLOCK))
        .map(|i| i.arg(0))
        .collect::<Result<_, _>>()?;
    let pointers: HashSet<u32> = m
        .insts
        .iter()
        .filter(|i| i.op == OP_TYPE_POINTER && i.args.get(1) == Some(&STORAGE_UNIFORM))
        .filter(|i| i.args.get(2).is_some_and(|t| blocks.contains(t)))
        .map(|i| i.arg(0))
        .collect::<Result<_, _>>()?;
    let vars: Vec<(u32, u32)> = m
        .insts
        .iter()
        .filter(|i| i.op == OP_VARIABLE && i.args.first().is_some_and(|t| pointers.contains(t)))
        .map(|i| Ok((i.arg(0)?, i.arg(1)?)))
        .collect::<Result<_, ShaderError>>()?;
    let [(ptr, var)] = vars[..] else { return Ok(words.to_vec()) };
    let spares = [m.new_id(), m.new_id()];
    let mut out = Vec::with_capacity(m.insts.len() + 8);
    let mut named = false;
    for inst in std::mem::take(&mut m.insts) {
        let (op, first) = (inst.op, inst.args.first().copied());
        let decoration = (op == OP_DECORATE && first == Some(var)).then(|| inst.args.clone());
        out.push(inst);
        match op {
            OP_NAME if first == Some(var) => {
                named = true;
                for (id, name) in spares.iter().zip(SPARE_BLOCKS) {
                    let mut a = vec![*id];
                    a.extend(string_words(name));
                    out.push(Inst::new(OP_NAME, a));
                }
            }
            OP_VARIABLE if first == Some(ptr) => {
                for id in spares {
                    out.push(Inst::new(OP_VARIABLE, vec![ptr, id, STORAGE_UNIFORM]));
                }
            }
            _ => {}
        }
        if let Some(args) = decoration
            && matches!(args.get(1), Some(&(DECORATION_BINDING | DECORATION_DESCRIPTOR_SET)))
        {
            for id in spares {
                let mut a = args.clone();
                a[0] = id;
                out.push(Inst::new(OP_DECORATE, a));
            }
        }
    }
    // (Unnamed, the spares couldn't be told apart in the IR: none.)
    if !named {
        return Ok(words.to_vec());
    }
    m.insts = out;
    Ok(m.words())
}

/// How many locations a type takes (32-bit types: one per vector, column
/// and element).
fn slots(id: u32, types: &HashMap<u32, Inst>, consts: &HashMap<u32, u32>, depth: u32) -> u32 {
    let Some(t) = types.get(&id).filter(|_| depth < 16) else { return 1 };
    let arg = |i: usize| t.args.get(i).copied().unwrap_or(0);
    match t.op {
        OP_TYPE_MATRIX => arg(2).max(1) * slots(arg(1), types, consts, depth + 1),
        OP_TYPE_ARRAY => {
            consts.get(&arg(2)).copied().unwrap_or(1).max(1)
                * slots(arg(1), types, consts, depth + 1)
        }
        OP_TYPE_STRUCT => t.args[1..].iter().map(|&m| slots(m, types, consts, depth + 1)).sum(),
        _ => 1,
    }
}

/// An input or output at a location, with the name the shader gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Io {
    pub(crate) output: bool,
    pub(crate) name: String,
    pub(crate) location: u32,
}

/// Gives every input and output without a location or a built-in one,
/// inputs and outputs counted apart, in declaration order; and lists them
/// all by name (naga keeps an output's name nowhere).
pub(crate) fn assign_locations(words: &[u32]) -> Result<(Vec<u32>, Vec<Io>), ShaderError> {
    let mut m = Module::parse(words)?;
    let mut names: HashMap<u32, String> = HashMap::new();
    let mut builtin: HashSet<u32> = HashSet::new();
    let mut located: HashMap<u32, u32> = HashMap::new();
    let mut builtin_structs: HashSet<u32> = HashSet::new();
    let mut types: HashMap<u32, Inst> = HashMap::new();
    let mut consts: HashMap<u32, u32> = HashMap::new();
    for inst in &m.insts {
        match inst.op {
            OP_NAME => {
                names.insert(inst.arg(0)?, string_at(&inst.args, 1));
            }
            OP_DECORATE => match inst.arg(1)? {
                DECORATION_LOCATION => {
                    located.insert(inst.arg(0)?, inst.arg(2)?);
                }
                DECORATION_BUILTIN => {
                    builtin.insert(inst.arg(0)?);
                }
                _ => {}
            },
            OP_MEMBER_DECORATE if inst.arg(2)? == DECORATION_BUILTIN => {
                builtin_structs.insert(inst.arg(0)?);
            }
            OP_TYPE_BOOL | OP_TYPE_INT | OP_TYPE_FLOAT | OP_TYPE_VECTOR | OP_TYPE_MATRIX
            | OP_TYPE_ARRAY | OP_TYPE_STRUCT | OP_TYPE_POINTER => {
                types.insert(inst.arg(0)?, inst.clone());
            }
            OP_CONSTANT => {
                consts.insert(inst.arg(1)?, inst.arg(2)?);
            }
            _ => {}
        }
    }
    // Inputs (0) and outputs (1) that need one, and where the new ones
    // start: past those given in the source (`layout(location = 0)`).
    let mut next = [0u32; 2];
    let mut open = Vec::new();
    let mut io = Vec::new();
    let name = |id: u32| names.get(&id).cloned().unwrap_or_default();
    for inst in m.insts.iter().filter(|i| i.op == OP_VARIABLE) {
        let (ptr, id, storage) = (inst.arg(0)?, inst.arg(1)?, inst.arg(2)?);
        let which = match storage {
            STORAGE_INPUT => 0,
            STORAGE_OUTPUT => 1,
            _ => continue,
        };
        let pointee = types.get(&ptr).and_then(|p| p.args.get(2).copied()).unwrap_or(0);
        let size = slots(pointee, &types, &consts, 0);
        if let Some(&at) = located.get(&id) {
            next[which] = next[which].max(at.saturating_add(size));
            io.push(Io { output: which == 1, name: name(id), location: at });
        } else if !builtin.contains(&id) && !builtin_structs.contains(&pointee) {
            open.push((which, id, size));
        }
    }
    if open.is_empty() {
        return Ok((words.to_vec(), io));
    }
    let mut added = Vec::new();
    for (which, id, size) in open {
        let at = next[which];
        next[which] = at.saturating_add(size);
        added.push(Inst::new(OP_DECORATE, vec![id, DECORATION_LOCATION, at]));
        io.push(Io { output: which == 1, name: name(id), location: at });
    }
    let first = m.insts.iter().position(Inst::is_declaration).unwrap_or(m.insts.len());
    m.insts.splice(first..first, added);
    Ok((m.words(), io))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_round_trip() {
        let w = string_words("texUnit0");
        assert_eq!(w.len(), 3);
        assert_eq!(string_at(&w, 0), "texUnit0");
        assert_eq!(string_words("abc").len(), 1);
        assert_eq!(string_words("abcd").len(), 2);
    }

    #[test]
    fn bad_words_are_errors() {
        assert!(split_samplers(&[]).is_err());
        assert!(split_samplers(&[1, 2, 3, 4, 5]).is_err());
        // An instruction claiming more words than there are.
        assert!(split_samplers(&[MAGIC, 0x10000, 10, 0, 0, (9 << 16) | 5, 1]).is_err());
        // A zero word count.
        assert!(assign_locations(&[MAGIC, 0x10000, 10, 0, 0, 0]).is_err());
        // A too-short instruction is an error, not a panic.
        assert!(split_samplers(&[MAGIC, 0x10000, 10, 0, 0, (1 << 16) | 27]).is_err());
        assert!(split_samplers(&[MAGIC, 0x10000, 10, 0, 0, (2 << 16) | 27, 3]).is_err());
        assert!(assign_locations(&[MAGIC, 0x10000, 10, 0, 0, (2 << 16) | 71, 3]).is_err());
        // (Too short to be what the pass looks for: passed over, no panic.)
        for op in [71, 32, 59, 5] {
            let _ = add_spare_blocks(&[MAGIC, 0x10000, 10, 0, 0, (2 << 16) | op, 3]);
        }
        assert!(add_spare_blocks(&[1, 2]).is_err());
        assert!(add_spare_blocks(&[MAGIC, 0x10000, 10, 0, 0, (9 << 16) | 59, 1]).is_err());
    }
}
