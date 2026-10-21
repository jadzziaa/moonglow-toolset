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
//! - **The lights by index** ([`index_lights`]): a read of the engine's
//!   `lightPosition[i]` or `lightColor[i]` becomes a read of the frame's
//!   lights at the draw's `i`th index, so that a draw's lights are a list
//!   of numbers rather than its lights' values.
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
const OP_MEMBER_NAME: u16 = 6;
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
const OP_ENTRY_POINT: u16 = 15;
const OP_TYPE_RUNTIME_ARRAY: u16 = 29;
const OP_FUNCTION: u16 = 54;
const OP_FUNCTION_END: u16 = 56;
const OP_STORE: u16 = 62;
const OP_ACCESS_CHAIN: u16 = 65;
const OP_IN_BOUNDS_ACCESS_CHAIN: u16 = 66;
const OP_BITCAST: u16 = 124;
const OP_IADD: u16 = 128;
const OP_IMUL: u16 = 132;
const OP_SHIFT_RIGHT_LOGICAL: u16 = 194;
const OP_BITWISE_AND: u16 = 199;
const OP_LABEL: u16 = 248;

const DECORATION_BUILTIN: u32 = 11;
const DECORATION_LOCATION: u32 = 30;
const DECORATION_BINDING: u32 = 33;
const DECORATION_DESCRIPTOR_SET: u32 = 34;

const DECORATION_BLOCK: u32 = 2;
const DECORATION_BUFFER_BLOCK: u32 = 3;
const DECORATION_ARRAY_STRIDE: u32 = 6;
const DECORATION_FLAT: u32 = 14;
const DECORATION_NON_WRITABLE: u32 = 24;
const DECORATION_OFFSET: u32 = 35;
const BUILTIN_INSTANCE_INDEX: u32 = 43;
const MODEL_VERTEX: u32 = 0;

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

/// The engine's arrays of its lights' values, each a `vec4` a light
/// (`inc_lighting`'s `lightPosition[MAX_NUM_LIGHTS]`, view-space positions,
/// and `lightColor`), which [`index_lights`] reads from [`LIGHT_TABLE`]:
/// a light's position is its place there times two, its colour the place
/// after.
pub const LIGHT_ARRAYS: [&str; 2] = ["lightPosition", "lightColor"];
/// The member [`index_lights`] adds to the uniform block: the draw's
/// lights' places in [`LIGHT_TABLE`], four a `uvec4`, as many as the
/// arrays have lights.
pub const LIGHT_INDEX: &str = "mg_lightIndex";
/// The storage buffer of the frame's lights' values (`vec4`s, two a light,
/// as [`LIGHT_ARRAYS`] has them).
pub const LIGHT_TABLE: &str = "mg_light_table";

/// Reads the engine's lights ([`LIGHT_ARRAYS`]) by index. The uniform
/// block gets a member of the draw's lights' places ([`LIGHT_INDEX`]), and
/// each read of `lightPosition[i]` or `lightColor[i]` (and of a component
/// of one) becomes a read of [`LIGHT_TABLE`] at the `i`th place, times two
/// (plus one for the colour). The arrays themselves are then read nowhere
/// ([`crate::Program::split_uniforms`] leaves them out). What the program
/// computes is the same: the same values, found another way. A program
/// that reads the arrays otherwise than an element at a time (a whole
/// array copied), or whose arrays aren't of `vec4`s, is left as it is, as
/// is one that reads neither.
pub(crate) fn index_lights(words: &[u32]) -> Result<Vec<u32>, ShaderError> {
    let mut m = Module::parse(words)?;
    // The one uniform block, its type and variable (as `add_spare_blocks`).
    let blocks: HashSet<u32> = m
        .insts
        .iter()
        .filter(|i| i.op == OP_DECORATE && i.args.get(1) == Some(&DECORATION_BLOCK))
        .map(|i| i.arg(0))
        .collect::<Result<_, _>>()?;
    let mut block = None;
    let mut vars = Vec::new();
    for i in m.insts.iter().filter(|i| i.op == OP_TYPE_POINTER) {
        if i.arg(1)? == STORAGE_UNIFORM && blocks.contains(&i.arg(2)?) {
            let ptr = i.arg(0)?;
            for v in m.insts.iter().filter(|v| v.op == OP_VARIABLE) {
                if v.arg(0)? == ptr {
                    vars.push(v.arg(1)?);
                    block = Some(i.arg(2)?);
                }
            }
        }
    }
    let (Some(block), &[var]) = (block, &vars[..]) else { return Ok(words.to_vec()) };
    // Its light arrays: member index, which (0 position, 1 colour), length.
    let mut consts: HashMap<u32, u32> = HashMap::new();
    let mut types: HashMap<u32, Inst> = HashMap::new();
    for i in &m.insts {
        match i.op {
            OP_CONSTANT => {
                consts.insert(i.arg(1)?, i.arg(2)?);
            }
            // (Types only: a constant's first operand is its type.)
            19..=39 => {
                if let Some(&id) = i.args.first() {
                    types.insert(id, i.clone());
                }
            }
            _ => {}
        }
    }
    let Some(st) =
        m.insts.iter().position(|i| i.op == OP_TYPE_STRUCT && i.args.first() == Some(&block))
    else {
        return Ok(words.to_vec());
    };
    let member_types: Vec<u32> = m.insts[st].args[1..].to_vec();
    let is = |id: u32, op: u16, rest: &[u32]| {
        types.get(&id).is_some_and(|t| t.op == op && t.args.get(1..) == Some(rest))
    };
    let mut arrays: Vec<(u32, u32, u32)> = Vec::new();
    for i in m.insts.iter().filter(|i| i.op == OP_MEMBER_NAME && i.args.first() == Some(&block)) {
        let (k, name) = (i.arg(1)?, string_at(&i.args, 2));
        let Some(which) = LIGHT_ARRAYS.iter().position(|a| *a == name) else { continue };
        let ty = member_types.get(k as usize).copied().unwrap_or(0);
        let Some(t) = types.get(&ty).filter(|t| t.op == OP_TYPE_ARRAY) else {
            return Ok(words.to_vec());
        };
        let (element, length) = (t.arg(1)?, t.arg(2)?);
        let vec4 = types.get(&element).is_some_and(|v| {
            v.op == OP_TYPE_VECTOR
                && v.args.get(2) == Some(&4)
                && v.args.get(1).is_some_and(|&f| is(f, OP_TYPE_FLOAT, &[32]))
        });
        let Some(&length) = consts.get(&length).filter(|_| vec4) else {
            return Ok(words.to_vec());
        };
        arrays.push((k, which as u32, length));
    }
    // The accesses to them: each an element's (or a component of one's).
    let mut reads = 0;
    for i in &m.insts {
        if matches!(i.op, OP_ACCESS_CHAIN | OP_IN_BOUNDS_ACCESS_CHAIN)
            && i.args.get(2) == Some(&var)
            && let Some(k) = i.args.get(3).and_then(|c| consts.get(c))
            && arrays.iter().any(|a| a.0 == *k)
        {
            if i.args.len() < 5 {
                return Ok(words.to_vec());
            }
            reads += 1;
        }
    }
    // (The block itself used otherwise is the split's to refuse.)
    if reads == 0 {
        return Ok(words.to_vec());
    }
    // The type of each value a function computes, and of each constant:
    // an element's index may be signed or not.
    let mut result_type: HashMap<u32, u32> = HashMap::new();
    let mut in_fn = false;
    for i in &m.insts {
        match i.op {
            OP_FUNCTION => in_fn = true,
            OP_FUNCTION_END => in_fn = false,
            _ => {}
        }
        let constant = matches!(i.op, OP_CONSTANT | 50 /* OpSpecConstant */);
        if (constant || (in_fn && has_result_type(i.op))) && i.args.len() >= 2 {
            result_type.insert(i.args[1], i.args[0]);
        }
    }
    // The types the reads need, declared first: moved there where the
    // module has them later (a scalar type depends on nothing, a vector on
    // its scalar, moved before it).
    let mut decls: Vec<Inst> = Vec::new();
    let hoist = |m: &mut Module, decls: &mut Vec<Inst>, op: u16, rest: Vec<u32>| match m
        .insts
        .iter()
        .position(|i| i.op == op && i.args.get(1..) == Some(&rest[..]))
    {
        Some(at) => {
            let i = m.insts.remove(at);
            let id = i.args[0];
            decls.push(i);
            id
        }
        None => {
            let id = m.new_id();
            let mut a = vec![id];
            a.extend(rest);
            decls.push(Inst::new(op, a));
            id
        }
    };
    let u32_t = hoist(&mut m, &mut decls, OP_TYPE_INT, vec![32, 0]);
    let uvec4 = hoist(&mut m, &mut decls, OP_TYPE_VECTOR, vec![u32_t, 4]);
    let f32_t = hoist(&mut m, &mut decls, OP_TYPE_FLOAT, vec![32]);
    let vec4 = hoist(&mut m, &mut decls, OP_TYPE_VECTOR, vec![f32_t, 4]);
    let int_t = m
        .insts
        .iter()
        .find(|i| i.op == OP_TYPE_INT && i.args.get(1..) == Some(&[32, 1][..]))
        .map(|i| i.args[0]);
    let constant = |m: &mut Module, decls: &mut Vec<Inst>, v: u32| {
        let id = m.new_id();
        decls.push(Inst::new(OP_CONSTANT, vec![u32_t, id, v]));
        id
    };
    let index_member = member_types.len() as u32;
    let lights = arrays.iter().map(|a| a.2).max().unwrap_or(0).max(1);
    let c: Vec<u32> = [0, 1, 2, 3, index_member, lights.div_ceil(4)]
        .iter()
        .map(|&v| constant(&mut m, &mut decls, v))
        .collect();
    let (c0, c1, c2, c3, c_member, c_count) = (c[0], c[1], c[2], c[3], c[4], c[5]);
    let index_t = m.new_id();
    decls.push(Inst::new(OP_TYPE_ARRAY, vec![index_t, uvec4, c_count]));
    let u32_ptr = m.new_id();
    decls.push(Inst::new(OP_TYPE_POINTER, vec![u32_ptr, STORAGE_UNIFORM, u32_t]));
    let (table_array, table_t, table_ptr, table) = (m.new_id(), m.new_id(), m.new_id(), m.new_id());
    decls.push(Inst::new(OP_TYPE_RUNTIME_ARRAY, vec![table_array, vec4]));
    decls.push(Inst::new(OP_TYPE_STRUCT, vec![table_t, table_array]));
    decls.push(Inst::new(OP_TYPE_POINTER, vec![table_ptr, STORAGE_UNIFORM, table_t]));
    decls.push(Inst::new(OP_VARIABLE, vec![table_ptr, table, STORAGE_UNIFORM]));
    let mut annotations = vec![
        Inst::new(OP_DECORATE, vec![index_t, DECORATION_ARRAY_STRIDE, 16]),
        Inst::new(OP_DECORATE, vec![table_array, DECORATION_ARRAY_STRIDE, 16]),
        Inst::new(OP_DECORATE, vec![table_t, DECORATION_BUFFER_BLOCK]),
        Inst::new(OP_MEMBER_DECORATE, vec![table_t, 0, DECORATION_OFFSET, 0]),
        Inst::new(OP_MEMBER_DECORATE, vec![table_t, 0, DECORATION_NON_WRITABLE]),
        Inst::new(OP_DECORATE, vec![table, DECORATION_DESCRIPTOR_SET, 0]),
        Inst::new(OP_DECORATE, vec![table, DECORATION_BINDING, 0]),
    ];
    // The new member, past the block's others.
    let size = |t: u32| block_size(t, &types, &consts, 0);
    let mut end = 0u32;
    for i in m.insts.iter().filter(|i| i.op == OP_MEMBER_DECORATE && i.args.first() == Some(&block))
    {
        if i.arg(2)? == DECORATION_OFFSET {
            let k = i.arg(1)? as usize;
            let ty = member_types.get(k).copied().unwrap_or(0);
            end = end.max(i.arg(3)?.saturating_add(size(ty)));
        }
    }
    annotations.push(Inst::new(
        OP_MEMBER_DECORATE,
        vec![block, index_member, DECORATION_OFFSET, end.div_ceil(16) * 16],
    ));
    let mut names = Vec::new();
    let mut a = vec![block, index_member];
    a.extend(string_words(LIGHT_INDEX));
    names.push(Inst::new(OP_MEMBER_NAME, a));
    let mut a = vec![table];
    a.extend(string_words(LIGHT_TABLE));
    names.push(Inst::new(OP_NAME, a));

    let first_annotation = m
        .insts
        .iter()
        .position(|i| matches!(i.op, OP_DECORATE | OP_MEMBER_DECORATE) || i.is_declaration())
        .unwrap_or(m.insts.len());
    let first_decl = m.insts.iter().position(Inst::is_declaration).unwrap_or(m.insts.len());
    let mut out = Vec::with_capacity(m.insts.len() + decls.len() + 8 * reads + 16);
    for (n, inst) in std::mem::take(&mut m.insts).into_iter().enumerate() {
        if n == first_annotation {
            out.append(&mut names);
        }
        if n == first_decl {
            out.append(&mut annotations);
            out.append(&mut decls);
        }
        match inst.op {
            OP_TYPE_STRUCT if inst.args.first() == Some(&block) => {
                let mut inst = inst;
                inst.args.push(index_t);
                out.push(inst);
            }
            OP_ACCESS_CHAIN | OP_IN_BOUNDS_ACCESS_CHAIN
                if inst.args.get(2) == Some(&var)
                    && let Some(k) = inst.args.get(3).and_then(|c| consts.get(c))
                    && let Some(&(_, which, _)) = arrays.iter().find(|a| a.0 == *k) =>
            {
                // The element's index, unsigned.
                let i = inst.arg(4)?;
                let i = match result_type.get(&i) {
                    Some(&t) if t == u32_t => i,
                    Some(&t) if Some(t) == int_t => {
                        let u = m.new_id();
                        out.push(Inst::new(OP_BITCAST, vec![u32_t, u, i]));
                        u
                    }
                    _ => return Err(bad("a light's index that isn't a 32-bit integer")),
                };
                // Its place: the index's member, its `uvec4` and component.
                let (q, r, p, l, d, t) =
                    (m.new_id(), m.new_id(), m.new_id(), m.new_id(), m.new_id(), m.new_id());
                out.push(Inst::new(OP_SHIFT_RIGHT_LOGICAL, vec![u32_t, q, i, c2]));
                out.push(Inst::new(OP_BITWISE_AND, vec![u32_t, r, i, c3]));
                out.push(Inst::new(OP_ACCESS_CHAIN, vec![u32_ptr, p, var, c_member, q, r]));
                out.push(Inst::new(OP_LOAD, vec![u32_t, l, p]));
                out.push(Inst::new(OP_IMUL, vec![u32_t, d, l, c2]));
                let which = if which == 0 { c0 } else { c1 };
                out.push(Inst::new(OP_IADD, vec![u32_t, t, d, which]));
                // The table's element there (and its component).
                let mut a = vec![inst.args[0], inst.args[1], table, c0, t];
                a.extend(&inst.args[5..]);
                out.push(Inst::new(OP_ACCESS_CHAIN, a));
            }
            _ => out.push(inst),
        }
    }
    m.insts = out;
    Ok(m.words())
}

/// Whether an instruction in a function has a result type (its first
/// operand) and a result (its second): all but those that store, branch,
/// end or mark.
fn has_result_type(op: u16) -> bool {
    !matches!(
        op,
        0 // OpNop
            | 8 // OpLine
            | OP_FUNCTION_END
            | OP_STORE
            | 63 // OpCopyMemory
            | 64 // OpCopyMemorySized
            | 99 // OpImageWrite
            | 218..=221 // OpEmitVertex, OpEndPrimitive, the streams'
            | 224 | 225 // OpControlBarrier, OpMemoryBarrier
            | 246..=255 // merges, OpLabel, branches, OpKill, returns, OpUnreachable
            | 317 // OpNoLine
            | 4416 // OpTerminateInvocation
    )
}

/// How many bytes a uniform block's member of type `id` takes, at most
/// (std140's: a vector or scalar its components, a matrix a column of 16
/// bytes each, an array its length times its stride).
fn block_size(id: u32, types: &HashMap<u32, Inst>, consts: &HashMap<u32, u32>, depth: u32) -> u32 {
    let Some(t) = types.get(&id).filter(|_| depth < 16) else { return 16 };
    let arg = |i: usize| t.args.get(i).copied().unwrap_or(0);
    match t.op {
        OP_TYPE_BOOL | OP_TYPE_INT | OP_TYPE_FLOAT => 4,
        OP_TYPE_VECTOR => 4 * arg(2),
        OP_TYPE_MATRIX => 16 * arg(2),
        OP_TYPE_ARRAY => {
            let element = block_size(arg(1), types, consts, depth + 1).div_ceil(16) * 16;
            consts.get(&arg(2)).copied().unwrap_or(1).saturating_mul(element)
        }
        OP_TYPE_STRUCT => t.args[1..]
            .iter()
            .map(|&m| block_size(m, types, consts, depth + 1).div_ceil(16) * 16)
            .fold(0u32, u32::saturating_add),
        _ => 16,
    }
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

/// The storage buffers and inputs [`add_draw_index`] declares, by name: a
/// draw's own values, its lights', the vertex program's order of draws, and
/// the draw's and its lights' places passed to the fragment program.
pub const DRAWS: &str = "mg_draws";
pub const LIGHTS: &str = "mg_lights";
pub const ORDER: &str = "mg_order";
pub(crate) const DRAW_VARYING: &str = "mg_draw";
pub(crate) const LIGHT_VARYING: &str = "mg_light";

/// Declares what lets a renderer draw many draws of a program with one
/// call, each with its own values: storage buffers of a draw's values and
/// of its lights (arrays of `vec4`, given their element types in the IR by
/// [`crate::Program::split_uniforms`]), and in the vertex program the
/// order of draws, read at the instance's index for the draw's place in
/// each (the vertex program's draw and lights, the fragment program's,
/// handed to it as flat varyings). At the start of every function, the
/// places are read and a pointer taken to the draw's element of each array:
/// naga keeps those expressions before the function's own, so that the IR
/// can take every access to the program's uniforms there.
pub(crate) fn add_draw_index(words: &[u32]) -> Result<Vec<u32>, ShaderError> {
    let mut m = Module::parse(words)?;
    let entry = m.insts.iter().find(|i| i.op == OP_ENTRY_POINT).cloned();
    let Some(entry) = entry else { return Ok(words.to_vec()) };
    let vertex = entry.arg(0)? == MODEL_VERTEX;
    let main = entry.arg(1)?;
    // The types there are, and those to add.
    let mut decls: Vec<Inst> = Vec::new();
    let have = |m: &mut Module, decls: &mut Vec<Inst>, op: u16, rest: Vec<u32>| match find_in(
        m, decls, op, &rest,
    ) {
        Some(id) => id,
        None => {
            let id = m.new_id();
            let mut a = vec![id];
            a.extend(rest);
            decls.push(Inst::new(op, a));
            id
        }
    };
    let u32_t = have(&mut m, &mut decls, OP_TYPE_INT, vec![32, 0]);
    let f32_t = have(&mut m, &mut decls, OP_TYPE_FLOAT, vec![32]);
    let vec4 = have(&mut m, &mut decls, OP_TYPE_VECTOR, vec![f32_t, 4]);
    let uvec4 = have(&mut m, &mut decls, OP_TYPE_VECTOR, vec![u32_t, 4]);
    let consts: Vec<u32> = (0..4)
        .map(|v| {
            let id = m.new_id();
            decls.push(Inst::new(OP_CONSTANT, vec![u32_t, id, v]));
            id
        })
        .collect();
    let mut annotations: Vec<Inst> = Vec::new();
    let mut names: Vec<Inst> = Vec::new();
    let name = |names: &mut Vec<Inst>, id: u32, text: &str| {
        let mut a = vec![id];
        a.extend(string_words(text));
        names.push(Inst::new(OP_NAME, a));
    };
    // A storage buffer: a struct of one runtime array of `element`.
    let mut buffer = |m: &mut Module, decls: &mut Vec<Inst>, element: u32| {
        let (array, block) = (m.new_id(), m.new_id());
        decls.push(Inst::new(OP_TYPE_RUNTIME_ARRAY, vec![array, element]));
        decls.push(Inst::new(OP_TYPE_STRUCT, vec![block, array]));
        annotations.push(Inst::new(OP_DECORATE, vec![array, DECORATION_ARRAY_STRIDE, 16]));
        annotations.push(Inst::new(OP_DECORATE, vec![block, DECORATION_BUFFER_BLOCK]));
        annotations.push(Inst::new(OP_MEMBER_DECORATE, vec![block, 0, DECORATION_OFFSET, 0]));
        annotations.push(Inst::new(OP_MEMBER_DECORATE, vec![block, 0, DECORATION_NON_WRITABLE]));
        block
    };
    let draws_t = buffer(&mut m, &mut decls, vec4);
    let order_t = buffer(&mut m, &mut decls, uvec4);
    let ptr = |m: &mut Module, decls: &mut Vec<Inst>, class: u32, to: u32| {
        let id = m.new_id();
        decls.push(Inst::new(OP_TYPE_POINTER, vec![id, class, to]));
        id
    };
    let draws_ptr = ptr(&mut m, &mut decls, STORAGE_UNIFORM, draws_t);
    let order_ptr = ptr(&mut m, &mut decls, STORAGE_UNIFORM, order_t);
    let vec4_ptr = ptr(&mut m, &mut decls, STORAGE_UNIFORM, vec4);
    let u32_ptr = ptr(&mut m, &mut decls, STORAGE_UNIFORM, u32_t);
    let in_ptr = ptr(&mut m, &mut decls, STORAGE_INPUT, u32_t);
    let out_ptr = ptr(&mut m, &mut decls, STORAGE_OUTPUT, u32_t);
    let var = |m: &mut Module,
               decls: &mut Vec<Inst>,
               names: &mut Vec<Inst>,
               ty: u32,
               class: u32,
               text: &str| {
        let id = m.new_id();
        decls.push(Inst::new(OP_VARIABLE, vec![ty, id, class]));
        name(names, id, text);
        id
    };
    let draws = var(&mut m, &mut decls, &mut names, draws_ptr, STORAGE_UNIFORM, DRAWS);
    let lights = var(&mut m, &mut decls, &mut names, draws_ptr, STORAGE_UNIFORM, LIGHTS);
    for v in [draws, lights] {
        annotations.push(Inst::new(OP_DECORATE, vec![v, DECORATION_DESCRIPTOR_SET, 0]));
        annotations.push(Inst::new(OP_DECORATE, vec![v, DECORATION_BINDING, 0]));
    }
    // The places: (in the vertex program) read from the order at the
    // instance's index, the fragment program's passed on; (in the
    // fragment program) as they come.
    let mut interface = Vec::new();
    let mut start: Vec<Inst> = Vec::new();
    let mut at_main: Vec<Inst> = Vec::new();
    let (draw_at, light_at);
    if vertex {
        let order = var(&mut m, &mut decls, &mut names, order_ptr, STORAGE_UNIFORM, ORDER);
        annotations.push(Inst::new(OP_DECORATE, vec![order, DECORATION_DESCRIPTOR_SET, 0]));
        annotations.push(Inst::new(OP_DECORATE, vec![order, DECORATION_BINDING, 0]));
        // (glslang may have declared the instance's index: `gl_InstanceID`
        // is it, less the first instance.)
        let declared = m
            .insts
            .iter()
            .find(|i| {
                i.op == OP_DECORATE
                    && i.args.get(1) == Some(&DECORATION_BUILTIN)
                    && i.args.get(2) == Some(&BUILTIN_INSTANCE_INDEX)
            })
            .map(|i| i.args[0]);
        let (instance, instance_t) = match declared {
            Some(v) => {
                // Its type: the pointee of its variable's pointer.
                let ptr = m
                    .insts
                    .iter()
                    .find(|i| i.op == OP_VARIABLE && i.args.get(1) == Some(&v))
                    .map(|i| i.arg(0))
                    .ok_or_else(|| bad("the instance's index isn't a variable"))??;
                let t = m
                    .insts
                    .iter()
                    .find(|i| i.op == OP_TYPE_POINTER && i.args.first() == Some(&ptr))
                    .map(|i| i.arg(2))
                    .ok_or_else(|| bad("the instance's index has no type"))??;
                (v, t)
            }
            None => {
                let v = var(&mut m, &mut decls, &mut names, in_ptr, STORAGE_INPUT, "mg_instance");
                annotations.push(Inst::new(
                    OP_DECORATE,
                    vec![v, DECORATION_BUILTIN, BUILTIN_INSTANCE_INDEX],
                ));
                interface.push(v);
                (v, u32_t)
            }
        };
        let outs: Vec<u32> = [DRAW_VARYING, LIGHT_VARYING]
            .iter()
            .map(|n| {
                let v = var(&mut m, &mut decls, &mut names, out_ptr, STORAGE_OUTPUT, n);
                annotations.push(Inst::new(OP_DECORATE, vec![v, DECORATION_FLAT]));
                v
            })
            .collect();
        interface.extend(&outs);
        let i = m.new_id();
        start.push(Inst::new(OP_LOAD, vec![instance_t, i, instance]));
        let place = |m: &mut Module, start: &mut Vec<Inst>, which: usize| {
            let (p, v) = (m.new_id(), m.new_id());
            start.push(Inst::new(
                OP_ACCESS_CHAIN,
                vec![u32_ptr, p, order, consts[0], i, consts[which]],
            ));
            start.push(Inst::new(OP_LOAD, vec![u32_t, v, p]));
            v
        };
        // (Each a draw's: the vertex program's draw and lights, the
        // fragment program's draw and lights.)
        let (vd, vl, fd, fl) = (
            place(&mut m, &mut start, 0),
            place(&mut m, &mut start, 2),
            place(&mut m, &mut start, 1),
            place(&mut m, &mut start, 3),
        );
        at_main.push(Inst::new(OP_STORE, vec![outs[0], fd]));
        at_main.push(Inst::new(OP_STORE, vec![outs[1], fl]));
        (draw_at, light_at) = (vd, vl);
    } else {
        let ins: Vec<u32> = [DRAW_VARYING, LIGHT_VARYING]
            .iter()
            .map(|n| {
                let v = var(&mut m, &mut decls, &mut names, in_ptr, STORAGE_INPUT, n);
                annotations.push(Inst::new(OP_DECORATE, vec![v, DECORATION_FLAT]));
                v
            })
            .collect();
        interface.extend(&ins);
        let (d, l) = (m.new_id(), m.new_id());
        start.push(Inst::new(OP_LOAD, vec![u32_t, d, ins[0]]));
        start.push(Inst::new(OP_LOAD, vec![u32_t, l, ins[1]]));
        (draw_at, light_at) = (d, l);
    }
    let (pd, pl) = (m.new_id(), m.new_id());
    start.push(Inst::new(OP_ACCESS_CHAIN, vec![vec4_ptr, pd, draws, consts[0], draw_at]));
    start.push(Inst::new(OP_ACCESS_CHAIN, vec![vec4_ptr, pl, lights, consts[0], light_at]));

    // Placed: names before the annotations, annotations before the
    // declarations, the new declarations after the others; the reads at
    // the start of each function (after its variables); ids in each
    // function its own (the reads' results renumbered per function).
    let first_annotation = m
        .insts
        .iter()
        .position(|i| matches!(i.op, OP_DECORATE | OP_MEMBER_DECORATE) || i.is_declaration())
        .unwrap_or(m.insts.len());
    let first_decl = m.insts.iter().position(Inst::is_declaration).unwrap_or(m.insts.len());
    let first_fn = m.insts.iter().position(|i| i.op == OP_FUNCTION).unwrap_or(m.insts.len());
    let mut out = Vec::with_capacity(m.insts.len() + decls.len() + 64);
    let insts = std::mem::take(&mut m.insts);
    let mut in_fn: Option<u32> = None;
    let mut labelled = false;
    let mut placed = false;
    for (n, inst) in insts.into_iter().enumerate() {
        if n == first_annotation {
            out.append(&mut names);
        }
        if n == first_decl {
            out.append(&mut annotations);
        }
        if n == first_fn {
            out.append(&mut decls);
        }
        match inst.op {
            OP_ENTRY_POINT => {
                let mut inst = inst;
                inst.args.extend(&interface);
                out.push(inst);
                continue;
            }
            OP_FUNCTION => {
                in_fn = Some(inst.arg(1)?);
                (labelled, placed) = (false, false);
            }
            OP_LABEL if in_fn.is_some() && !labelled => {
                labelled = true;
                out.push(inst);
                continue;
            }
            OP_FUNCTION_END => in_fn = None,
            _ => {}
        }
        if labelled && !placed && inst.op != OP_VARIABLE {
            placed = true;
            // (Each function its own ids for the reads.)
            let mut map: HashMap<u32, u32> = HashMap::new();
            let fresh = |m: &mut Module, map: &mut HashMap<u32, u32>, id: u32| {
                *map.entry(id).or_insert_with(|| m.new_id())
            };
            let renumber = |m: &mut Module, map: &mut HashMap<u32, u32>, i: &Inst| {
                let mut i = i.clone();
                match i.op {
                    OP_LOAD | OP_ACCESS_CHAIN => {
                        i.args[1] = fresh(m, map, i.args[1]);
                        for a in i.args.iter_mut().skip(2) {
                            if let Some(&n) = map.get(a) {
                                *a = n;
                            }
                        }
                    }
                    OP_STORE => {
                        if let Some(&n) = map.get(&i.args[1]) {
                            i.args[1] = n;
                        }
                    }
                    _ => {}
                }
                i
            };
            let mine: Vec<Inst> = start.iter().map(|i| renumber(&mut m, &mut map, i)).collect();
            out.extend(mine);
            if in_fn == Some(main) {
                let stores: Vec<Inst> =
                    at_main.iter().map(|i| renumber(&mut m, &mut map, i)).collect();
                out.extend(stores);
            }
        }
        out.push(inst);
    }
    m.insts = out;
    Ok(m.words())
}

/// A type among the module's declarations and those to add.
fn find_in(m: &Module, decls: &[Inst], op: u16, rest: &[u32]) -> Option<u32> {
    m.insts
        .iter()
        .chain(decls)
        .find(|i| i.op == op && i.args.get(1..) == Some(rest))
        .map(|i| i.args[0])
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
        assert!(index_lights(&[1, 2]).is_err());
        assert!(index_lights(&[MAGIC, 0x10000, 10, 0, 0, (9 << 16) | 59, 1]).is_err());
        for op in [OP_DECORATE, OP_TYPE_POINTER, OP_VARIABLE, OP_TYPE_STRUCT, OP_MEMBER_NAME] {
            let _ = index_lights(&[MAGIC, 0x10000, 10, 0, 0, (2 << 16) | u32::from(op), 3]);
            let _ = index_lights(&[MAGIC, 0x10000, 10, 0, 0, (1 << 16) | u32::from(op)]);
        }
        assert!(add_spare_blocks(&[MAGIC, 0x10000, 10, 0, 0, (9 << 16) | 59, 1]).is_err());
    }
}
