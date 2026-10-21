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

use crate::reflect::{Scalar, ValueType, Varying};
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

    /// Puts every resource in bind group `group` (a pipeline's two
    /// programs each bind their own block at binding 0).
    pub fn set_group(&mut self, group: u32) -> Result<(), ShaderError> {
        for (_, g) in self.module.global_variables.iter_mut() {
            if let Some(b) = &mut g.binding {
                b.group = group;
            }
        }
        self.reflection.group = group;
        self.revalidate()
    }
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
