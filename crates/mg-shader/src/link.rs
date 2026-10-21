//! A program fitted to wgpu, in naga's IR: the clip space, an attribute
//! read in another type, the varyings matched by name between the stages,
//! the bind group (`docs/shaders-proposal.md`, M1).
//!
//! naga's SPIR-V reader wraps a program's `main` in an entry point that
//! stores each input argument into a private variable, calls `main`, then
//! loads the outputs and returns them: the passes here edit that wrapper.

use naga::proc::Emitter;
use naga::{
    BinaryOperator, Binding, Block, BuiltIn, Expression, Literal, Module, ScalarKind, Span,
    Statement, Type, TypeInner,
};

use crate::reflect::{Part, Scalar, UniformBlock, ValueType, Varying};
use crate::{Program, ShaderError};

/// The game's shaders write OpenGL's clip space: depth from -w to w. wgpu's
/// runs from 0 to w, so each vertex program's position is given
/// `z = (z + w) / 2` as it leaves (what `glDepthRange(0, 1)` makes of it:
/// `gl_FragCoord.z` is the same in both). Both have y up, so nothing is
/// flipped (naga's `adjust_coordinate_space` is for SPIR-V written for
/// Vulkan, whose y points down).
pub(crate) fn gl_depth_to_wgpu(module: &mut Module) -> Result<(), String> {
    let types = &mut module.types;
    for ep in module.entry_points.iter_mut().filter(|e| e.stage == naga::ShaderStage::Vertex) {
        let f = &mut ep.function;
        let Some(result) = &f.result else { continue };
        let is_position =
            |b: &Option<Binding>| matches!(b, Some(Binding::BuiltIn(BuiltIn::Position { .. })));
        // The position: the result itself, or which member of it.
        let member = if is_position(&result.binding) {
            None
        } else {
            match &types[result.ty].inner {
                TypeInner::Struct { members, .. } => {
                    match members.iter().position(|m| is_position(&m.binding)) {
                        Some(i) => Some((i, members[i].ty)),
                        None => continue,
                    }
                }
                _ => continue,
            }
        };
        let pos_ty = member.map_or(result.ty, |(_, ty)| ty);
        let at = f
            .body
            .iter()
            .rposition(|s| matches!(s, Statement::Return { value: Some(_) }))
            .ok_or("the vertex entry point returns nothing")?;
        let Statement::Return { value: Some(value) } = f.body[at] else { unreachable!() };
        let e = &mut f.expressions;
        let pos = match member {
            None => value,
            Some((i, _)) => match &e[value] {
                Expression::Compose { components, .. } if components.len() > i => components[i],
                _ => return Err("the vertex entry point's result isn't composed".into()),
            },
        };
        let half = e.append(Expression::Literal(Literal::F32(0.5)), Span::UNDEFINED);
        let mut emitter = Emitter::default();
        emitter.start(e);
        let c = |e: &mut naga::Arena<Expression>, index| {
            e.append(Expression::AccessIndex { base: pos, index }, Span::UNDEFINED)
        };
        let (x, y, z, w) = (c(e, 0), c(e, 1), c(e, 2), c(e, 3));
        let sum = e.append(
            Expression::Binary { op: BinaryOperator::Add, left: z, right: w },
            Span::UNDEFINED,
        );
        let z = e.append(
            Expression::Binary { op: BinaryOperator::Multiply, left: sum, right: half },
            Span::UNDEFINED,
        );
        let new_pos = e.append(
            Expression::Compose { ty: pos_ty, components: vec![x, y, z, w] },
            Span::UNDEFINED,
        );
        let new_value = match member {
            None => new_pos,
            Some((i, _)) => {
                let Expression::Compose { ty, components } = &e[value] else { unreachable!() };
                let mut components = components.clone();
                components[i] = new_pos;
                let ty = *ty;
                e.append(Expression::Compose { ty, components }, Span::UNDEFINED)
            }
        };
        let emit = emitter.finish(e).map(|(s, _)| s);
        f.body[at] = Statement::Return { value: Some(new_value) };
        if let Some(emit) = emit {
            f.body.splice(at..at, Block::from_vec(vec![emit]));
        }
    }
    Ok(())
}

/// Which fragments [`Program::keep_alpha`] keeps: those whose alpha is at
/// least the value, or below it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlphaKeep {
    AtLeast(f32),
    Below(f32),
}

fn naga_err(what: impl std::fmt::Display) -> ShaderError {
    ShaderError::Naga(what.to_string())
}

impl Program {
    /// Validates the module again after a pass changed it.
    fn revalidate(&mut self) -> Result<(), ShaderError> {
        self.info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&self.module)
        .map_err(|e| naga_err(crate::chain(e.as_inner())))?;
        Ok(())
    }

    /// Reads vertex input `name` from unsigned integers, converted to the
    /// floats the shader declares: the game's bone indices (`vIndex`, a
    /// `vec4`) from Moonglow's `u32`s, which no vertex format turns into
    /// floats. `false` where the program has no such float input.
    pub fn read_input_as_uint(&mut self, name: &str) -> Result<bool, ShaderError> {
        let Some(input) = self.reflection.inputs.iter_mut().find(|v| v.name == name) else {
            return Ok(false);
        };
        let size = match input.ty {
            ValueType::Float => None,
            ValueType::Vector(n, Scalar::Float) => Some(n),
            _ => return Ok(false),
        };
        let location = input.location;
        let ep = self.module.entry_points.first_mut().ok_or_else(|| naga_err("no entry point"))?;
        let f = &mut ep.function;
        let index = f
            .arguments
            .iter()
            .position(|a| {
                matches!(a.binding, Some(Binding::Location { location: l, .. }) if l == location)
            })
            .ok_or_else(|| naga_err(format!("no input at location {location}")))?;
        let scalar = naga::Scalar::U32;
        let inner = match size {
            None => TypeInner::Scalar(scalar),
            Some(n) => TypeInner::Vector {
                size: match n {
                    2 => naga::VectorSize::Bi,
                    3 => naga::VectorSize::Tri,
                    _ => naga::VectorSize::Quad,
                },
                scalar,
            },
        };
        f.arguments[index].ty =
            self.module.types.insert(Type { name: None, inner }, Span::UNDEFINED);
        // The store of the argument into its private variable takes the
        // argument converted.
        let arg = f
            .expressions
            .iter()
            .find(|(_, e)| matches!(e, Expression::FunctionArgument(i) if *i as usize == index))
            .map(|(h, _)| h)
            .ok_or_else(|| naga_err(format!("input {name} is never read")))?;
        let at = f
            .body
            .iter()
            .position(|s| matches!(s, Statement::Store { value, .. } if *value == arg))
            .ok_or_else(|| naga_err(format!("input {name} is never stored")))?;
        let mut emitter = Emitter::default();
        emitter.start(&f.expressions);
        let converted = f.expressions.append(
            Expression::As { expr: arg, kind: ScalarKind::Float, convert: Some(4) },
            Span::UNDEFINED,
        );
        let emit = emitter.finish(&f.expressions).map(|(s, _)| s);
        if let Statement::Store { value, .. } = &mut f.body[at] {
            *value = converted;
        }
        f.body.splice(at..at, Block::from_vec(emit.into_iter().collect()));
        self.revalidate()?;
        let input = self.reflection.inputs.iter_mut().find(|v| v.name == name).expect("found");
        input.ty = match size {
            None => ValueType::Uint,
            Some(n) => ValueType::Vector(n, Scalar::Uint),
        };
        Ok(true)
    }

    /// Gives a fragment program's inputs the locations of the vertex
    /// program's outputs of the same names: glslang numbers each stage's in
    /// its own order of declaration, and two varyings of one type swapped
    /// would draw garbage without an error. An input the vertex program
    /// doesn't write, or writes in another type, is an error.
    pub fn link_inputs(&mut self, vertex_outputs: &[Varying]) -> Result<(), ShaderError> {
        let mut moves = Vec::new();
        for input in &self.reflection.inputs {
            let out = vertex_outputs.iter().find(|o| o.name == input.name).ok_or_else(|| {
                naga_err(format!("varying {} isn't written by the vertex program", input.name))
            })?;
            if out.ty != input.ty {
                return Err(naga_err(format!(
                    "varying {} is {:?} in the vertex program and {:?} here",
                    input.name, out.ty, input.ty
                )));
            }
            moves.push((input.location, out.location));
        }
        let ep = self.module.entry_points.first_mut().ok_or_else(|| naga_err("no entry point"))?;
        for a in &mut ep.function.arguments {
            if let Some(Binding::Location { location, .. }) = &mut a.binding
                && let Some(&(_, to)) = moves.iter().find(|(from, _)| from == location)
            {
                *location = to;
            }
        }
        for v in &mut self.reflection.inputs {
            if let Some(&(_, to)) = moves.iter().find(|(from, _)| *from == v.location) {
                v.location = to;
            }
        }
        self.reflection.inputs.sort_by_key(|v| v.location);
        self.revalidate()
    }

    /// Keeps only the fragments whose color (the output at location 0, the
    /// game's `gl_FragColor`) has an alpha `keep` takes, discarding the
    /// rest as they leave: one part of a see-through mesh drawn in two
    /// (Moonglow's own split, `docs/shaders-proposal.md` M2: what is
    /// nearly opaque of it first, hiding what is behind, then the rest).
    pub fn keep_alpha(&mut self, keep: AlphaKeep) -> Result<(), ShaderError> {
        let types = &self.module.types;
        let ep = self
            .module
            .entry_points
            .iter_mut()
            .find(|e| e.stage == naga::ShaderStage::Fragment)
            .ok_or_else(|| naga_err("no fragment entry point"))?;
        let f = &mut ep.function;
        let result = f.result.as_ref().ok_or_else(|| naga_err("the program writes no color"))?;
        let is_color =
            |b: &Option<Binding>| matches!(b, Some(Binding::Location { location: 0, .. }));
        // The color: the result itself, or which member of it.
        let (member, ty) = if is_color(&result.binding) {
            (None, result.ty)
        } else {
            match &types[result.ty].inner {
                TypeInner::Struct { members, .. } => {
                    let i = members
                        .iter()
                        .position(|m| is_color(&m.binding))
                        .ok_or_else(|| naga_err("the program writes no color at location 0"))?;
                    (Some(i), members[i].ty)
                }
                _ => return Err(naga_err("the program writes no color at location 0")),
            }
        };
        if !matches!(
            types[ty].inner,
            TypeInner::Vector { size: naga::VectorSize::Quad, scalar: naga::Scalar::F32 }
        ) {
            return Err(naga_err("the color isn't a vec4"));
        }
        let at = f
            .body
            .iter()
            .rposition(|s| matches!(s, Statement::Return { value: Some(_) }))
            .ok_or_else(|| naga_err("the fragment entry point returns nothing"))?;
        let Statement::Return { value: Some(value) } = f.body[at] else { unreachable!() };
        let e = &mut f.expressions;
        let color = match member {
            None => value,
            Some(i) => match &e[value] {
                Expression::Compose { components, .. } if components.len() > i => components[i],
                _ => return Err(naga_err("the fragment entry point's result isn't composed")),
            },
        };
        let (op, threshold) = match keep {
            // (Discarded: what is below it.)
            AlphaKeep::AtLeast(t) => (BinaryOperator::Less, t),
            AlphaKeep::Below(t) => (BinaryOperator::GreaterEqual, t),
        };
        let limit = e.append(Expression::Literal(Literal::F32(threshold)), Span::UNDEFINED);
        let mut emitter = Emitter::default();
        emitter.start(e);
        let alpha = e.append(Expression::AccessIndex { base: color, index: 3 }, Span::UNDEFINED);
        let condition =
            e.append(Expression::Binary { op, left: alpha, right: limit }, Span::UNDEFINED);
        let mut statements: Vec<Statement> =
            emitter.finish(e).map(|(s, _)| s).into_iter().collect();
        statements.push(Statement::If {
            condition,
            accept: Block::from_vec(vec![Statement::Kill]),
            reject: Block::new(),
        });
        f.body.splice(at..at, Block::from_vec(statements));
        self.revalidate()
    }

    /// Whether the program reads uniform `name`: glslang keeps every loose
    /// uniform the program declares in its block (the stock includes
    /// declare `worldtimerTimeOfDay` for every program), read or not.
    pub fn reads_uniform(&self, name: &str) -> bool {
        let m = &self.module;
        // The blocks' variables, and the member's place in each.
        let mut members = Vec::new();
        for (h, g) in m.global_variables.iter() {
            if g.space != naga::AddressSpace::Uniform {
                continue;
            }
            if let TypeInner::Struct { members: list, .. } = &m.types[g.ty].inner
                && let Some(i) = list.iter().position(|x| x.name.as_deref() == Some(name))
            {
                members.push((h, i as u32));
            }
        }
        let reads = |f: &naga::Function| {
            f.expressions.iter().any(|(_, e)| match e {
                Expression::AccessIndex { base, index } => matches!(
                    f.expressions[*base],
                    Expression::GlobalVariable(g) if members.contains(&(g, *index))
                ),
                _ => false,
            })
        };
        !members.is_empty()
            && (m.functions.iter().any(|(_, f)| reads(f))
                || m.entry_points.iter().any(|e| reads(&e.function)))
    }

    /// Puts every resource in bind group `group` (a pipeline's two
    /// programs each bind their own block at binding 0).
    pub fn set_group(&mut self, group: u32) -> Result<(), ShaderError> {
        for (_, g) in self.module.global_variables.iter_mut() {
            if let Some(b) = &mut g.binding {
                b.group = group;
            }
        }
        self.reflection.group = group;
        for b in &mut self.reflection.uniforms {
            b.group = group;
        }
        self.revalidate()
    }

    /// Splits the program's uniform block into three by what each member
    /// is (`part_of`, which says [`Part::Draw`], [`Part::Frame`] or
    /// [`Part::Bones`]): one a renderer writes for each draw, one written
    /// once a frame (the projection, the fog, the clock: the same for
    /// every draw), one of the bones (a skinned draw's). Each is laid out
    /// anew, its members in the order they came in, so a draw's block is
    /// as small as its own members make it (`vslit`'s whole block is 8.7
    /// KB, 4 KB of it `m_bones`). A part with no members holds one unused
    /// `vec4`. The draw's block keeps the block's binding; the frame's and
    /// the bones' take the two that were kept for them (bound after the
    /// rest).
    ///
    /// `false`, the program unchanged, where it has no one block to split
    /// (no loose uniforms) or uses it otherwise than member by member.
    pub fn split_uniforms(
        &mut self,
        part_of: impl Fn(&crate::Member) -> Part,
    ) -> Result<bool, ShaderError> {
        let mut module = self.module.clone();
        let Some(blocks) = split_module(&mut module, &part_of)? else { return Ok(false) };
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .map_err(|e| naga_err(crate::chain(e.as_inner())))?;
        let group = self.reflection.group;
        let name = self.reflection.uniforms.first().map(|b| b.name.clone()).unwrap_or_default();
        self.reflection.uniforms = blocks
            .into_iter()
            .map(|(part, h)| {
                let g = &module.global_variables[h];
                let (members, size) = match &module.types[g.ty].inner {
                    TypeInner::Struct { members, span } => (
                        members
                            .iter()
                            .map(|m| crate::reflect::member(&module, m))
                            .filter(|m| m.name != PAD)
                            .collect(),
                        *span,
                    ),
                    _ => (Vec::new(), 16),
                };
                UniformBlock {
                    name: name.clone(),
                    part,
                    group,
                    binding: g.binding.map_or(0, |b| b.binding),
                    size,
                    members,
                }
            })
            .collect();
        self.module = module;
        self.info = info;
        Ok(true)
    }

    /// Binds the block of `part` at `group` and `binding` (a renderer's
    /// own layout: the draws' blocks of both stages in one group).
    pub fn place_block(&mut self, part: Part, group: u32, binding: u32) -> Result<(), ShaderError> {
        let Some(b) = self.reflection.uniforms.iter_mut().find(|b| b.part == part) else {
            return Err(naga_err(format!("no {part:?} block")));
        };
        let at = naga::ResourceBinding { group: b.group, binding: b.binding };
        let g = self
            .module
            .global_variables
            .iter_mut()
            .find(|(_, g)| g.space == naga::AddressSpace::Uniform && g.binding == Some(at))
            .ok_or_else(|| naga_err(format!("the {part:?} block isn't bound")))?
            .1;
        g.binding = Some(naga::ResourceBinding { group, binding });
        (b.group, b.binding) = (group, binding);
        self.revalidate()
    }
}

/// The member a part with none holds.
const PAD: &str = "mg_unused";

/// [`Program::split_uniforms`] on a module: the three blocks' variables, or
/// `None` where there is no one block to split.
#[allow(clippy::type_complexity)]
fn split_module(
    module: &mut Module,
    part_of: &dyn Fn(&crate::Member) -> Part,
) -> Result<Option<Vec<(Part, naga::Handle<naga::GlobalVariable>)>>, ShaderError> {
    use naga::{AddressSpace, Handle, StructMember};
    let named = |name: &str| {
        module
            .global_variables
            .iter()
            .find(|(_, g)| g.space == AddressSpace::Uniform && g.name.as_deref() == Some(name))
            .map(|(h, _)| h)
    };
    let [frame_name, bones_name] = crate::spirv::SPARE_BLOCKS;
    let (Some(frame), Some(bones)) = (named(frame_name), named(bones_name)) else {
        return Ok(None);
    };
    let blocks: Vec<Handle<naga::GlobalVariable>> = module
        .global_variables
        .iter()
        .filter(|(h, g)| g.space == AddressSpace::Uniform && *h != frame && *h != bones)
        .map(|(h, _)| h)
        .collect();
    let [block] = blocks[..] else { return Ok(None) };
    let ty = module.global_variables[block].ty;
    let TypeInner::Struct { members, .. } = module.types[ty].inner.clone() else {
        return Ok(None);
    };
    let block_name = module.types[ty].name.clone().unwrap_or_default();
    // Every use of the block is a member's access (as glslang writes them).
    for f in module
        .functions
        .iter()
        .map(|(_, f)| f)
        .chain(module.entry_points.iter().map(|e| &e.function))
    {
        let mine: Vec<_> = f
            .expressions
            .iter()
            .filter(|(_, e)| matches!(e, Expression::GlobalVariable(g) if *g == block))
            .map(|(h, _)| h)
            .collect();
        let other = f.expressions.iter().any(|(_, e)| match e {
            Expression::Load { pointer } => mine.contains(pointer),
            Expression::Access { base, .. } => mine.contains(base),
            _ => false,
        });
        if other {
            return Ok(None);
        }
    }
    // Each part's members, laid out anew.
    let mut layouter = naga::proc::Layouter::default();
    layouter.update(module.to_ctx()).map_err(naga_err)?;
    let order = [Part::Draw, Part::Frame, Part::Bones];
    let mut parts: [Vec<StructMember>; 3] = Default::default();
    let mut ends = [0u32; 3];
    let mut moved = Vec::with_capacity(members.len());
    for m in &members {
        let part = match part_of(&crate::reflect::member(module, m)) {
            Part::Whole => Part::Draw,
            p => p,
        };
        let k = order.iter().position(|p| *p == part).unwrap_or(0);
        let inner = &module.types[m.ty].inner;
        // (std140's: a vec2 by 8, a vec3, vec4, matrix or array by 16.)
        let own = match inner {
            TypeInner::Scalar(_) => 4,
            TypeInner::Vector { size: naga::VectorSize::Bi, .. } => 8,
            _ => 16,
        };
        let align = own.max(layouter[m.ty].alignment * 1);
        let offset = ends[k].div_ceil(align) * align;
        ends[k] = offset + inner.size(module.to_ctx());
        moved.push((k, parts[k].len() as u32));
        parts[k].push(StructMember { offset, ..m.clone() });
    }
    let vec4 = module.types.insert(
        naga::Type {
            name: None,
            inner: TypeInner::Vector { size: naga::VectorSize::Quad, scalar: naga::Scalar::F32 },
        },
        Span::UNDEFINED,
    );
    let globals = [block, frame, bones];
    for (k, list) in parts.into_iter().enumerate() {
        let mut list = list;
        if list.is_empty() {
            list.push(StructMember { name: Some(PAD.into()), ty: vec4, binding: None, offset: 0 });
            ends[k] = 16;
        }
        let span = ends[k].max(16).div_ceil(16) * 16;
        let t = module.types.insert(
            naga::Type {
                name: Some(format!("{block_name}_{:?}", order[k]).to_ascii_lowercase()),
                inner: TypeInner::Struct { members: list, span },
            },
            Span::UNDEFINED,
        );
        module.global_variables[globals[k]].ty = t;
    }
    // Each access to a member goes to its part's block.
    let fns = module
        .functions
        .iter_mut()
        .map(|(_, f)| f)
        .chain(module.entry_points.iter_mut().map(|e| &mut e.function));
    for f in fns {
        let find = |g: Handle<naga::GlobalVariable>| {
            f.expressions
                .iter()
                .find(|(_, e)| matches!(e, Expression::GlobalVariable(x) if *x == g))
                .map(|(h, _)| h)
        };
        let olds: Vec<Handle<Expression>> = f
            .expressions
            .iter()
            .filter(|(_, e)| matches!(e, Expression::GlobalVariable(x) if *x == block))
            .map(|(h, _)| h)
            .collect();
        if olds.is_empty() {
            continue;
        }
        let (Some(fe), Some(be)) = (find(frame), find(bones)) else {
            return Err(naga_err("a function without the spare blocks' expressions"));
        };
        let bases = [olds[0], fe, be];
        for (_, e) in f.expressions.iter_mut() {
            if let Expression::AccessIndex { base, index } = e
                && olds.contains(base)
            {
                let &(k, j) = moved
                    .get(*index as usize)
                    .ok_or_else(|| naga_err("an access past the block's members"))?;
                // (The draw's part stays on the expression it was read by.)
                if k != 0 {
                    *base = bases[k];
                }
                *index = j;
            }
        }
    }
    Ok(Some(order.into_iter().zip(globals).collect()))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{Preamble, Stage, compile};

    /// Two varyings of one type, used in one order by the vertex program
    /// and in another by the fragment program (glslang numbers them as they
    /// are first used); bone indices as floats, as the game declares them.
    fn sources() -> BTreeMap<String, String> {
        let vs = r#"
#define SHADER_TYPE 1
uniform mat4 m_mvp;
uniform mat4 m_bones[4];
attribute vec4 vPos;
attribute vec4 vIndex;
attribute vec2 vTcIn;
varying vec2 vFirst;
varying vec2 vSecond;
varying vec3 vThird;
void main ()
{
    vFirst = vTcIn;
    vSecond = vTcIn * 2.0;
    vThird = vPos.xyz;
    gl_Position = m_mvp * (m_bones[int(vIndex.y)] * vPos);
}
"#;
        let fs = r#"
#define SHADER_TYPE 2
uniform sampler2D texUnit0;
varying vec3 vThird;
varying vec2 vSecond;
varying vec2 vFirst;
void main ()
{
    vec4 c = vec4(vThird, 1.0);
    c += vec4(vSecond, 0.0, 0.0);
    gl_FragColor = c * texture2D(texUnit0, vFirst);
}
"#;
        let fs_missing = "#define SHADER_TYPE 2\nvarying vec2 vFourth;\n\
            void main() { gl_FragColor = vec4(vFourth, 0.0, 1.0); }\n";
        let fs_other = "#define SHADER_TYPE 2\nvarying vec3 vFirst;\n\
            void main() { gl_FragColor = vec4(vFirst, 1.0); }\n";
        [("vs_link", vs), ("fs_link", fs), ("fs_missing", fs_missing), ("fs_other", fs_other)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn program(name: &str, stage: Stage) -> Program {
        compile(name, stage, &Preamble::default(), &sources()).unwrap()
    }

    /// The position leaves as (x, y, (z + w) / 2, w): OpenGL's depth range
    /// taken to wgpu's, y as it was.
    #[test]
    fn the_position_s_depth_is_taken_to_wgpu_s_range() {
        let p = program("vs_link", Stage::Vertex);
        let f = &p.module.entry_points[0].function;
        let e = &f.expressions;
        let Some(Statement::Return { value: Some(v) }) = f.body.last() else { panic!() };
        let Expression::Compose { components, .. } = &e[*v] else { panic!("{:?}", e[*v]) };
        let pos = components
            .iter()
            .find_map(|&c| match &e[c] {
                Expression::Compose { components, .. } if components.len() == 4 => {
                    Some(components.clone())
                }
                _ => None,
            })
            .expect("a position composed anew");
        let index = |h: naga::Handle<Expression>| match e[h] {
            Expression::AccessIndex { index, .. } => Some(index),
            _ => None,
        };
        assert_eq!((index(pos[0]), index(pos[1]), index(pos[3])), (Some(0), Some(1), Some(3)));
        let Expression::Binary { op: BinaryOperator::Multiply, left, right } = e[pos[2]] else {
            panic!("{:?}", e[pos[2]])
        };
        assert_eq!(e[right], Expression::Literal(Literal::F32(0.5)));
        let Expression::Binary { op: BinaryOperator::Add, left: z, right: w } = e[left] else {
            panic!()
        };
        assert_eq!((index(z), index(w)), (Some(2), Some(3)));
    }

    #[test]
    fn varyings_are_matched_by_name() {
        let vs = program("vs_link", Stage::Vertex);
        let mut fs = program("fs_link", Stage::Fragment);
        // (As compiled, the stages disagree: each numbers its own.)
        let at = |v: &[Varying], name: &str| v.iter().find(|v| v.name == name).unwrap().location;
        assert_ne!(at(&fs.reflection.inputs, "vFirst"), at(&vs.reflection.outputs, "vFirst"));
        fs.link_inputs(&vs.reflection.outputs).unwrap();
        for input in &fs.reflection.inputs {
            let out = vs.reflection.outputs.iter().find(|o| o.name == input.name).unwrap();
            assert_eq!(input.location, out.location, "{}", input.name);
        }
        // The module's own arguments say the same.
        let ep = &fs.module.entry_points[0];
        let mut locations: Vec<u32> = ep
            .function
            .arguments
            .iter()
            .filter_map(|a| match a.binding {
                Some(Binding::Location { location, .. }) => Some(location),
                _ => None,
            })
            .collect();
        locations.sort();
        let mut want: Vec<u32> = fs.reflection.inputs.iter().map(|v| v.location).collect();
        want.sort();
        assert_eq!(locations, want);

        let e = program("fs_missing", Stage::Fragment).link_inputs(&vs.reflection.outputs);
        assert!(e.unwrap_err().to_string().contains("vFourth"));
        let e = program("fs_other", Stage::Fragment).link_inputs(&vs.reflection.outputs);
        assert!(e.unwrap_err().to_string().contains("vFirst"));
    }

    #[test]
    fn bone_indices_are_read_from_unsigned_integers() {
        let mut vs = program("vs_link", Stage::Vertex);
        assert!(vs.read_input_as_uint("vIndex").unwrap());
        let input = vs.reflection.inputs.iter().find(|v| v.name == "vIndex").unwrap();
        assert_eq!(input.ty, ValueType::Vector(4, Scalar::Uint));
        let arg = vs.module.entry_points[0]
            .function
            .arguments
            .iter()
            .find(|a| {
                matches!(a.binding, Some(Binding::Location { location, .. }) if location == input.location)
            })
            .unwrap();
        assert!(matches!(
            vs.module.types[arg.ty].inner,
            TypeInner::Vector { scalar: naga::Scalar::U32, .. }
        ));
        // Not a float input, or none: nothing done.
        assert!(!vs.read_input_as_uint("vIndex").unwrap());
        assert!(!vs.read_input_as_uint("vNothing").unwrap());
        naga::back::wgsl::write_string(
            &vs.module,
            &vs.info,
            naga::back::wgsl::WriterFlags::empty(),
        )
        .unwrap();
    }

    /// A see-through mesh's two parts: the color's alpha tested as it
    /// leaves, the fragment discarded below (or at and above) the value.
    #[test]
    fn fragments_are_kept_by_their_alpha() {
        for (keep, op) in [
            (AlphaKeep::AtLeast(0.95), BinaryOperator::Less),
            (AlphaKeep::Below(0.95), BinaryOperator::GreaterEqual),
        ] {
            let mut fs = program("fs_link", Stage::Fragment);
            fs.keep_alpha(keep).unwrap();
            let f = &fs.module.entry_points[0].function;
            let at = f.body.len() - 2;
            let Statement::If { condition, accept, .. } = &f.body[at] else {
                panic!("{:?}", f.body[at])
            };
            assert!(matches!(accept.first(), Some(Statement::Kill)));
            let e = &f.expressions;
            let Expression::Binary { op: o, left, right } = e[*condition] else { panic!() };
            assert_eq!(o, op);
            assert_eq!(e[right], Expression::Literal(Literal::F32(0.95)));
            assert!(matches!(e[left], Expression::AccessIndex { index: 3, .. }));
            naga::back::wgsl::write_string(
                &fs.module,
                &fs.info,
                naga::back::wgsl::WriterFlags::empty(),
            )
            .unwrap();
        }
        // A vertex program has no color to test.
        assert!(program("vs_link", Stage::Vertex).keep_alpha(AlphaKeep::Below(0.5)).is_err());
    }

    /// A uniform declared and not read is no uniform the program reads.
    #[test]
    fn uniforms_read_are_told_from_those_declared() {
        let src: BTreeMap<String, String> = [(
            "fs_time".to_string(),
            "#define SHADER_TYPE 2\nuniform int worldtimerTimeOfDay;\nuniform int sceneCurrentFrame;\n\
             uniform vec4 vColor;\nvoid main() { gl_FragColor = vColor * \
             float(worldtimerTimeOfDay); }\n"
                .to_string(),
        )]
        .into();
        let p = compile("fs_time", Stage::Fragment, &Preamble::default(), &src).unwrap();
        assert!(p.reflection.uniform("sceneCurrentFrame").is_some(), "declared, kept");
        assert!(p.reads_uniform("worldtimerTimeOfDay"));
        assert!(p.reads_uniform("vColor"));
        assert!(!p.reads_uniform("sceneCurrentFrame"));
        assert!(!p.reads_uniform("nothing"));
    }

    /// The block in three: a draw's members, the frame's, the bones', each
    /// laid out anew from 0; every access taken to its part's block.
    #[test]
    fn a_block_is_split_into_the_draw_s_the_frame_s_and_the_bones() {
        let src: BTreeMap<String, String> = [(
            "vs_split".to_string(),
            r#"
#define SHADER_TYPE 1
uniform mat4 m_proj;
uniform mat4 m_mv;
uniform mat4 m_bones[4];
uniform float fogEnd;
uniform vec3 lightAreaAmbient;
uniform vec4 materialFrontDiffuse;
uniform int skinmesh;
attribute vec4 vPos;
attribute vec4 vIndex;
varying vec4 vColor;
void main ()
{
    vec4 p = vPos;
    if (skinmesh != 0) p = m_bones[int(vIndex.x)] * p;
    vColor = materialFrontDiffuse * fogEnd + vec4(lightAreaAmbient, 0.0);
    gl_Position = m_proj * m_mv * p;
}
"#
            .to_string(),
        )]
        .into();
        let mut p = compile("vs_split", Stage::Vertex, &Preamble::default(), &src).unwrap();
        // As compiled: one block, the spare ones out of the reflection.
        assert_eq!(p.reflection.uniforms.len(), 1);
        assert_eq!(p.reflection.uniforms[0].part, Part::Whole);
        let part = |m: &crate::Member| match m.name.as_str() {
            "m_bones" => Part::Bones,
            "m_mv" | "materialFrontDiffuse" | "skinmesh" => Part::Draw,
            _ => Part::Frame,
        };
        assert!(p.split_uniforms(part).unwrap());
        let r = &p.reflection;
        let parts: Vec<Part> = r.uniforms.iter().map(|b| b.part).collect();
        assert_eq!(parts, [Part::Draw, Part::Frame, Part::Bones]);
        let names = |b: &UniformBlock| {
            b.members.iter().map(|m| (m.name.clone(), m.offset)).collect::<Vec<_>>()
        };
        let s = |v: &[(&str, u32)]| v.iter().map(|(n, o)| (n.to_string(), *o)).collect::<Vec<_>>();
        assert_eq!(
            names(&r.uniforms[0]),
            s(&[("m_mv", 0), ("materialFrontDiffuse", 64), ("skinmesh", 80)])
        );
        assert_eq!(r.uniforms[0].size, 96);
        assert_eq!(
            names(&r.uniforms[1]),
            s(&[("m_proj", 0), ("fogEnd", 64), ("lightAreaAmbient", 80)])
        );
        assert_eq!(names(&r.uniforms[2]), s(&[("m_bones", 0)]));
        assert_eq!(r.uniforms[2].size, 256);
        // Each its own binding.
        let mut b: Vec<u32> = r.uniforms.iter().map(|b| b.binding).collect();
        b.dedup();
        assert_eq!(b.len(), 3);
        // Moved to a group of the renderer's.
        p.place_block(Part::Draw, 2, 5).unwrap();
        let d = &p.reflection.uniforms[0];
        assert_eq!((d.group, d.binding), (2, 5));
        let wgsl = naga::back::wgsl::write_string(
            &p.module,
            &p.info,
            naga::back::wgsl::WriterFlags::empty(),
        )
        .unwrap();
        assert!(wgsl.contains("@group(2) @binding(5)"), "{wgsl}");
        // A program without loose uniforms has nothing to split.
        let src: BTreeMap<String, String> = [(
            "fs_plain".to_string(),
            "#define SHADER_TYPE 2\nvoid main() { gl_FragColor = vec4(1.0); }\n".to_string(),
        )]
        .into();
        let mut p = compile("fs_plain", Stage::Fragment, &Preamble::default(), &src).unwrap();
        assert!(!p.split_uniforms(part).unwrap());
    }

    #[test]
    fn a_program_s_resources_move_to_another_group() {
        let mut fs = program("fs_link", Stage::Fragment);
        fs.set_group(1).unwrap();
        assert_eq!(fs.reflection.group, 1);
        let groups: Vec<u32> = fs
            .module
            .global_variables
            .iter()
            .filter_map(|(_, g)| g.binding.map(|b| b.group))
            .collect();
        assert!(!groups.is_empty() && groups.iter().all(|&g| g == 1), "{groups:?}");
    }
}
