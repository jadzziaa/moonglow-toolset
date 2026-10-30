//! Compiled scripts (`.ncs`), read: their instructions, the strings they
//! push and the engine functions they call with constant arguments
//! (`EffectVisualEffect(VFX_IMP_HEALING_S)` is a `CONST.I 194` pushed just
//! before the `ACTION` that names `EffectVisualEffect`). What a module's
//! scripts reach by a number (a visual effect, a spell, an appearance) is
//! read here: the compiler has put every constant's value in, wherever it
//! was declared (`nwscript.nss`, an include, the script), and a hak's
//! script without its source is read the same way.
//!
//! The format is the game's virtual machine's (big-endian): `NCS V1.0`,
//! then an instruction `T` (0x42) with the file's size, then the code. Each
//! instruction is an opcode and a type byte, and the operands its opcode
//! and type take (`scriptinternal.h`'s `CVIRTUALMACHINE_OPCODE_*`, as the
//! compiler writes them in `scriptcompfinalcode.cpp`). Offsets are from the
//! code's start, as `nwn_asm` prints them.

use mg_core::bin::{BinError, Reader};
use thiserror::Error;

use crate::spec::Spec;

/// A compiled script that cannot be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NcsError {
    #[error("not a compiled script (no `NCS V1.0` header)")]
    NotNcs,
    #[error("at offset {offset}: {source}")]
    Truncated { offset: usize, source: BinError },
    #[error("at offset {offset}: unknown opcode {opcode:#04x} (type {ty:#04x})")]
    Unknown { offset: usize, opcode: u8, ty: u8 },
}

/// The code's start: the header (8 bytes) and the `T` instruction (5).
const CODE: usize = 13;

/// The type bytes the virtual machine knows (`CVIRTUALMACHINE_AUXCODE_*`).
pub mod ty {
    pub const INTEGER: u8 = 0x03;
    pub const FLOAT: u8 = 0x04;
    pub const STRING: u8 = 0x05;
    pub const OBJECT: u8 = 0x06;
    /// An engine structure, `ENGST0` (0x10) to `ENGST9` (0x19).
    pub const ENGST_LOCATION: u8 = 0x12;
    pub const ENGST_JSON: u8 = 0x17;
    pub const STRUCT_STRUCT: u8 = 0x24;
    pub const VECTOR_VECTOR: u8 = 0x3a;
    pub const VECTOR_FLOAT: u8 = 0x3b;
    pub const FLOAT_VECTOR: u8 = 0x3c;
}

/// An opcode (`CVIRTUALMACHINE_OPCODE_*`).
pub mod op {
    pub const CPDOWNSP: u8 = 0x01;
    pub const RSADD: u8 = 0x02;
    pub const CPTOPSP: u8 = 0x03;
    pub const CONST: u8 = 0x04;
    pub const ACTION: u8 = 0x05;
    pub const LOGAND: u8 = 0x06;
    pub const NEG: u8 = 0x19;
    pub const COMP: u8 = 0x1a;
    pub const MOVSP: u8 = 0x1b;
    pub const STORE_IP: u8 = 0x1c;
    pub const JMP: u8 = 0x1d;
    pub const JSR: u8 = 0x1e;
    pub const JZ: u8 = 0x1f;
    pub const RET: u8 = 0x20;
    pub const DESTRUCT: u8 = 0x21;
    pub const NOT: u8 = 0x22;
    pub const DECISP: u8 = 0x23;
    pub const INCISP: u8 = 0x24;
    pub const JNZ: u8 = 0x25;
    pub const CPDOWNBP: u8 = 0x26;
    pub const CPTOPBP: u8 = 0x27;
    pub const DECIBP: u8 = 0x28;
    pub const INCIBP: u8 = 0x29;
    pub const SAVEBP: u8 = 0x2a;
    pub const RESTOREBP: u8 = 0x2b;
    pub const STORE_STATE: u8 = 0x2c;
    pub const NOP: u8 = 0x2d;
    pub const EQUAL: u8 = 0x0b;
    pub const NEQUAL: u8 = 0x0c;
}

/// A constant pushed (`CONST`).
#[derive(Debug, Clone, PartialEq)]
pub enum Const {
    Int(i32),
    Float(f32),
    String(Vec<u8>),
    Object(u32),
    /// Another engine structure's constant (a location, a json), as its
    /// bytes.
    Other(Vec<u8>),
}

/// One instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct Instr {
    /// Where it is, from the code's start (as `nwn_asm` prints it).
    pub offset: usize,
    pub op: u8,
    pub ty: u8,
    pub arg: Arg,
}

/// An instruction's operands.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    None,
    Const(Const),
    /// `ACTION`: the engine function's id and how many arguments were
    /// pushed for it.
    Action {
        routine: u16,
        argc: u8,
    },
    /// A stack offset and a size (`CPDOWNSP`, `CPTOPSP` and the base
    /// pointer's).
    Copy {
        offset: i32,
        size: u16,
    },
    /// A jump's relative target, a `MOVSP`'s, or an increment's offset.
    Offset(i32),
    /// `EQUAL` and `NEQUAL` of two structures: their size.
    Size(u16),
    /// `DESTRUCT`: the size, the part kept's offset and its size.
    Destruct(u16, u16, u16),
    /// `STORE_STATE`: the base pointer's and the stack's sizes saved.
    State(i32, i32),
}

fn be<const N: usize>(r: &mut Reader<'_>) -> Result<[u8; N], BinError> {
    r.array::<N>()
}

fn i32_be(r: &mut Reader<'_>) -> Result<i32, BinError> {
    Ok(i32::from_be_bytes(be(r)?))
}

fn u16_be(r: &mut Reader<'_>) -> Result<u16, BinError> {
    Ok(u16::from_be_bytes(be(r)?))
}

/// The instructions of a compiled script.
pub fn decode(data: &[u8]) -> Result<Vec<Instr>, NcsError> {
    if data.len() < CODE || &data[..8] != b"NCS V1.0" || data[8] != 0x42 {
        return Err(NcsError::NotNcs);
    }
    let mut r = Reader::at(data, CODE).map_err(|_| NcsError::NotNcs)?;
    let mut out = Vec::new();
    while r.remaining() > 0 {
        let offset = data.len() - r.remaining() - CODE;
        let truncated = |source| NcsError::Truncated { offset, source };
        let [op, ty] = be::<2>(&mut r).map_err(truncated)?;
        let arg = operands(&mut r, op, ty).map_err(truncated)?.ok_or(NcsError::Unknown {
            offset,
            opcode: op,
            ty,
        })?;
        out.push(Instr { offset, op, ty, arg });
    }
    Ok(out)
}

/// An instruction's operands, `None` for an opcode not known.
fn operands(r: &mut Reader<'_>, op: u8, ty: u8) -> Result<Option<Arg>, BinError> {
    Ok(Some(match op {
        op::CPDOWNSP | op::CPTOPSP | op::CPDOWNBP | op::CPTOPBP => {
            Arg::Copy { offset: i32_be(r)?, size: u16_be(r)? }
        }
        op::RSADD | op::RET | op::NOT | op::SAVEBP | op::RESTOREBP | op::NOP => Arg::None,
        op::CONST => Arg::Const(match ty {
            ty::INTEGER => Const::Int(i32_be(r)?),
            ty::FLOAT => Const::Float(f32::from_be_bytes(be(r)?)),
            ty::STRING => {
                let n = u16_be(r)?;
                Const::String(r.bytes(n.into())?.to_vec())
            }
            ty::OBJECT => Const::Object(u32::from_be_bytes(be(r)?)),
            ty::ENGST_LOCATION => Const::Other(r.bytes(4)?.to_vec()),
            ty::ENGST_JSON => {
                let n = u16_be(r)?;
                Const::Other(r.bytes(n.into())?.to_vec())
            }
            0x10..=0x19 => Const::Other(Vec::new()),
            _ => return Ok(None),
        }),
        op::ACTION => Arg::Action { routine: u16_be(r)?, argc: be::<1>(r)?[0] },
        op::EQUAL | op::NEQUAL if ty == ty::STRUCT_STRUCT => Arg::Size(u16_be(r)?),
        op::LOGAND..=op::COMP => Arg::None,
        op::MOVSP | op::JMP | op::JSR | op::JZ | op::JNZ => Arg::Offset(i32_be(r)?),
        op::DECISP | op::INCISP | op::DECIBP | op::INCIBP => Arg::Offset(i32_be(r)?),
        op::STORE_IP => Arg::None,
        op::DESTRUCT => Arg::Destruct(u16_be(r)?, u16_be(r)?, u16_be(r)?),
        op::STORE_STATE => Arg::State(i32_be(r)?, i32_be(r)?),
        _ => return Ok(None),
    }))
}

/// The strings a script pushes as constants, in order.
pub fn strings(code: &[Instr]) -> impl Iterator<Item = &[u8]> {
    code.iter().filter_map(|i| match &i.arg {
        Arg::Const(Const::String(s)) => Some(&s[..]),
        _ => None,
    })
}

/// An engine function called, with the arguments that were constants.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub offset: usize,
    pub routine: u16,
    /// Per argument pushed (in the declaration's order), its constant;
    /// `None` where it was worked out as the script ran (a variable, a
    /// sum, another function's result) or could not be told.
    pub args: Vec<Option<Const>>,
}

/// The size on the stack of a value of an NWScript type, in bytes: a
/// vector's three floats, anything else one slot (an engine structure is
/// a handle).
pub fn type_size(ty: &str) -> usize {
    match ty {
        "void" => 0,
        "vector" => 12,
        _ => 4,
    }
}

/// The engine functions a script calls, each with its constant arguments
/// (`spec` gives each function's parameter types, by id). An argument is
/// told by going back from the call over what was pushed after it: a
/// constant (four bytes; a vector's three), a copy of a variable (its
/// size), another engine function's call (its result, in place of its own
/// arguments); anything else in the way, or a place another jump comes to,
/// leaves it untold.
pub fn calls(code: &[Instr], spec: &Spec) -> Vec<Call> {
    let targets = jump_targets(code);
    let mut out = Vec::new();
    for (at, i) in code.iter().enumerate() {
        let Arg::Action { routine, argc } = i.arg else { continue };
        let Some(f) = spec.functions.get(usize::from(routine)) else { continue };
        let mut args = Vec::with_capacity(argc.into());
        let mut depth = 0;
        for p in f.params.iter().take(argc.into()) {
            // (A vector is three constants, never one.)
            let one = type_size(&p.ty) == 4;
            args.push(one.then(|| pushed_at(code, at, depth, spec, &targets)).flatten());
            depth += type_size(&p.ty);
        }
        out.push(Call { offset: i.offset, routine, args });
    }
    out
}

/// The offsets jumps come to (the instructions after which code is not
/// straight).
fn jump_targets(code: &[Instr]) -> std::collections::HashSet<usize> {
    code.iter()
        .filter_map(|i| match (i.op, &i.arg) {
            (op::JMP | op::JSR | op::JZ | op::JNZ, Arg::Offset(d)) => {
                usize::try_from(i.offset as i64 + i64::from(*d)).ok()
            }
            // (A STORE_STATE saves the code after the next instruction, a
            // JMP over it: the code run later.)
            _ => None,
        })
        .collect()
}

/// The constant at `depth` bytes below the top of the stack as the
/// instruction at `at` runs, if a constant was pushed there in the
/// straight code before it.
fn pushed_at(
    code: &[Instr],
    at: usize,
    mut depth: usize,
    spec: &Spec,
    targets: &std::collections::HashSet<usize>,
) -> Option<Const> {
    let mut k = at;
    loop {
        // (Code a jump comes to may be reached from elsewhere than what
        // is before it.)
        if k == 0 || targets.contains(&code[k].offset) {
            return None;
        }
        k -= 1;
        let i = &code[k];
        // What it pushed (on top), and what it took off first.
        let (pushed, popped) = match (&i.arg, i.op) {
            (Arg::Const(c), _) => {
                if depth < 4 {
                    return (depth == 0).then(|| c.clone());
                }
                (4, 0)
            }
            (Arg::Copy { size, .. }, op::CPTOPSP | op::CPTOPBP) => (usize::from(*size), 0),
            (Arg::None, op::RSADD) => (4, 0),
            (Arg::Action { routine, argc }, _) => {
                let f = spec.functions.get(usize::from(*routine))?;
                let args: usize =
                    f.params.iter().take((*argc).into()).map(|p| type_size(&p.ty)).sum();
                (type_size(&f.return_type), args)
            }
            _ => return None,
        };
        if depth < pushed {
            return None;
        }
        depth = depth - pushed + popped;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ncs(code: &[u8]) -> Vec<u8> {
        let mut d = b"NCS V1.0".to_vec();
        d.push(0x42);
        d.extend_from_slice(&((CODE + code.len()) as u32).to_be_bytes());
        d.extend_from_slice(code);
        d
    }

    fn spec() -> Spec {
        Spec::parse(
            b"int GetIsPC(object oCreature);\n\
              effect EffectVisualEffect(int nVisualEffectId, int nMissEffect=FALSE, \
              vector vTargetOffset=[0.0,0.0,0.0]);\n\
              void SetCreatureAppearanceType(object oCreature, int nAppearanceType);\n\
              object GetEnteringObject();\n",
        )
    }

    /// `EffectVisualEffect(314)`: its first argument, pushed last, is the
    /// constant just before the call; the second, under it, too.
    #[test]
    fn a_call_s_constant_arguments() {
        let code = [
            &[0x04, 0x04][..],
            &0f32.to_be_bytes(),
            &[0x04, 0x04],
            &0f32.to_be_bytes(),
            &[0x04, 0x04],
            &0f32.to_be_bytes(),
            &[0x04, 0x03],
            &0i32.to_be_bytes(),
            &[0x04, 0x03],
            &314i32.to_be_bytes(),
            &[0x05, 0x00, 0x00, 0x01, 0x03],
        ]
        .concat();
        let code = decode(&ncs(&code)).unwrap();
        let calls = calls(&code, &spec());
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].routine, 1);
        assert_eq!(calls[0].args[0], Some(Const::Int(314)));
        assert_eq!(calls[0].args[1], Some(Const::Int(0)));
        assert_eq!(calls[0].args[2], None, "a vector is three floats: not one constant");
    }

    /// `SetCreatureAppearanceType(GetEnteringObject(), 7)`: the creature is
    /// a call's result (untold), the appearance under it a constant.
    #[test]
    fn past_another_call_s_result() {
        let code = [
            &[0x04, 0x03][..],
            &7i32.to_be_bytes(),
            &[0x05, 0x00, 0x00, 0x03, 0x00],
            &[0x05, 0x00, 0x00, 0x02, 0x02],
        ]
        .concat();
        let code = decode(&ncs(&code)).unwrap();
        let calls = calls(&code, &spec());
        let set = calls.iter().find(|c| c.routine == 2).unwrap();
        assert_eq!(set.args, vec![None, Some(Const::Int(7))]);
    }

    /// A variable's copy, or a jump's landing in between, leaves the
    /// argument untold.
    #[test]
    fn a_variable_or_a_jump_between_is_untold() {
        let copy = [&[0x03, 0x01][..], &(-8i32).to_be_bytes(), &4u16.to_be_bytes()].concat();
        let call = [0x05, 0x00, 0x00, 0x01, 0x01];
        let code = decode(&ncs(&[&copy[..], &call].concat())).unwrap();
        assert_eq!(calls(&code, &spec())[0].args, vec![None]);
        // The JMP (at 0) lands on the call (at 12), past the constant.
        let code =
            [&[0x1d, 0x00][..], &12i32.to_be_bytes(), &[0x04, 0x03], &5i32.to_be_bytes(), &call]
                .concat();
        let code = decode(&ncs(&code)).unwrap();
        assert_eq!(calls(&code, &spec())[0].args, vec![None]);
    }

    #[test]
    fn strings_pushed() {
        let code = [&[0x04, 0x05][..], &3u16.to_be_bytes(), b"abc"].concat();
        let code = decode(&ncs(&code)).unwrap();
        assert_eq!(strings(&code).collect::<Vec<_>>(), vec![&b"abc"[..]]);
    }

    /// Bad input is an error, never a panic: every truncation of a script.
    #[test]
    fn truncated_scripts_are_errors() {
        let code = [
            &[0x04, 0x05][..],
            &3u16.to_be_bytes(),
            b"abc",
            &[0x05, 0x00, 0x00, 0x01, 0x01],
            &[0x01, 0x01],
            &(-8i32).to_be_bytes(),
            &4u16.to_be_bytes(),
        ]
        .concat();
        let whole = ncs(&code);
        assert!(decode(&whole).is_ok());
        for n in 0..whole.len() {
            let _ = decode(&whole[..n]).map(|c| calls(&c, &spec()));
        }
        assert_eq!(decode(b"NCS V1.0"), Err(NcsError::NotNcs));
        assert!(matches!(decode(&ncs(&[0x77, 0x00])), Err(NcsError::Unknown { .. })));
    }
}
