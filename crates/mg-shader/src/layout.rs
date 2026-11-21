//! A block's layout as OpenGL lays it out. For wgpu's OpenGL back end
//! naga writes a uniform block as `layout(std140) uniform` and a storage
//! buffer as `layout(std430) buffer`, each member a plain GLSL type with
//! no offset and no stride (`naga::back::glsl` drops both): the driver
//! lays the members out by the rules alone. On Vulkan, Metal and DX12 the
//! offsets and strides of naga's module are written out and kept. So a
//! module whose offsets or strides differ from the rules draws right on
//! those and reads the wrong bytes on OpenGL: a storage element of one
//! `int` is 4 bytes wide by std430, so draw 1 reads byte 4 where the
//! renderer wrote it at byte 16.
//!
//! [`relayout`] gives a type the rules' layout (where naga can hold it),
//! and [`differs`] says where a type's layout is not the rules'.

use naga::{ArraySize, Handle, Module, Span, StructMember, Type, TypeInner, VectorSize};

/// GLSL's layout rules for a block's members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rules {
    /// A uniform block's: an array's stride and a struct's alignment are
    /// rounded up to 16 bytes.
    Std140,
    /// A storage buffer's: as std140, without that rounding.
    Std430,
}

/// The alignment and size of `ty` under `rules`, in bytes; `None` for a
/// type no block holds (an image, a sampler, a pointer).
pub(crate) fn of(module: &Module, ty: Handle<Type>, rules: Rules) -> Option<(u32, u32)> {
    let round = |a: u32| match rules {
        Rules::Std140 => a.div_ceil(16) * 16,
        Rules::Std430 => a,
    };
    Some(match &module.types[ty].inner {
        TypeInner::Scalar(s) | TypeInner::Atomic(s) => (u32::from(s.width), u32::from(s.width)),
        TypeInner::Vector { size, scalar } => vector(*size, u32::from(scalar.width)),
        TypeInner::Matrix { columns, rows, scalar } => {
            // (An array of its columns.)
            let (a, s) = vector(*rows, u32::from(scalar.width));
            let a = round(a);
            let stride = s.div_ceil(a) * a;
            (a, stride * *columns as u32)
        }
        TypeInner::Array { base, size, .. } => {
            let (a, s) = of(module, *base, rules)?;
            let a = round(a);
            let stride = s.div_ceil(a) * a;
            let n = match size {
                ArraySize::Constant(n) => n.get(),
                // (A runtime array: as one element, as naga takes it.)
                _ => 1,
            };
            (a, stride * n)
        }
        TypeInner::Struct { members, .. } => {
            let (mut align, mut end) = (round(4), 0u32);
            for m in members {
                let (a, s) = of(module, m.ty, rules)?;
                end = end.div_ceil(a) * a + s;
                align = align.max(a);
            }
            let align = round(align);
            (align, end.div_ceil(align) * align)
        }
        _ => return None,
    })
}

/// A vector's alignment and size: a `vec3`'s as a `vec4`'s alignment.
fn vector(size: VectorSize, width: u32) -> (u32, u32) {
    let n = size as u32;
    (if n == 2 { 2 * width } else { 4 * width }, n * width)
}

/// The stride of an array of `base` under `rules`.
fn stride(module: &Module, base: Handle<Type>, rules: Rules) -> Option<u32> {
    let (a, s) = of(module, base, rules)?;
    let a = match rules {
        Rules::Std140 => a.div_ceil(16) * 16,
        Rules::Std430 => a,
    };
    Some(s.div_ceil(a) * a)
}

/// Where `ty`'s layout in `module` is not `rules`' (a member's offset, an
/// array's stride, a struct's or a matrix's size), said with the path to
/// it from `path`; `None` where it is the same throughout.
pub(crate) fn differs(
    module: &Module,
    ty: Handle<Type>,
    rules: Rules,
    path: &str,
) -> Option<String> {
    let Some((_, size)) = of(module, ty, rules) else {
        return Some(format!("{path}: a type no block holds"));
    };
    match &module.types[ty].inner {
        TypeInner::Matrix { .. } => {
            let own = module.types[ty].inner.size(module.to_ctx());
            (own != size).then(|| format!("{path}: a matrix of {own} bytes, {size} by {rules:?}"))
        }
        TypeInner::Array { base, stride: own, .. } => match stride(module, *base, rules) {
            Some(want) if want != *own => {
                Some(format!("{path}: an array's stride of {own} bytes, {want} by {rules:?}"))
            }
            _ => differs(module, *base, rules, &format!("{path}[]")),
        },
        TypeInner::Struct { members, span } => members_differ(module, members, rules, path)
            .or_else(|| {
                (*span != size).then(|| format!("{path}: {span} bytes, {size} by {rules:?}"))
            }),
        _ => None,
    }
}

/// [`differs`] for a struct's members: each one's offset and type.
fn members_differ(
    module: &Module,
    members: &[StructMember],
    rules: Rules,
    path: &str,
) -> Option<String> {
    let mut end = 0u32;
    for (i, m) in members.iter().enumerate() {
        let name = match &m.name {
            Some(n) => format!("{path}.{n}"),
            None => format!("{path}.{i}"),
        };
        let Some((a, s)) = of(module, m.ty, rules) else {
            return Some(format!("{name}: a type no block holds"));
        };
        let want = end.div_ceil(a) * a;
        if m.offset != want {
            return Some(format!("{name}: at byte {}, {want} by {rules:?}", m.offset));
        }
        if let Some(why) = differs(module, m.ty, rules, &name) {
            return Some(why);
        }
        end = want + s;
    }
    None
}

/// `ty` with `rules`' layout: an array's stride and a struct's members'
/// offsets and size made the rules' (the types within first), a new type
/// where they differ, `ty` itself where they don't. A matrix is kept as
/// it is: naga holds a matrix's columns as std430 lays them out, and
/// [`differs`] finds a std140 one that differs.
pub(crate) fn relayout(module: &mut Module, ty: Handle<Type>, rules: Rules) -> Handle<Type> {
    let name = module.types[ty].name.clone();
    let inner = match module.types[ty].inner.clone() {
        TypeInner::Array { base, size, stride: own } => {
            let new_base = relayout(module, base, rules);
            let Some(new_stride) = stride(module, new_base, rules) else { return ty };
            if new_base == base && new_stride == own {
                return ty;
            }
            TypeInner::Array { base: new_base, size, stride: new_stride }
        }
        TypeInner::Struct { members, span } => {
            // (Its alignment is its members', which a stride doesn't change.)
            let Some((align, _)) = of(module, ty, rules) else { return ty };
            let mut list = Vec::with_capacity(members.len());
            let mut end = 0u32;
            for m in &members {
                let new_ty = relayout(module, m.ty, rules);
                let Some((a, s)) = of(module, new_ty, rules) else { return ty };
                let offset = end.div_ceil(a) * a;
                end = offset + s;
                list.push(StructMember { ty: new_ty, offset, ..m.clone() });
            }
            let size = end.div_ceil(align) * align;
            if list == members && size == span {
                return ty;
            }
            TypeInner::Struct { members: list, span: size }
        }
        _ => return ty,
    };
    module.types.insert(Type { name, inner }, Span::UNDEFINED)
}

/// Where one of `module`'s blocks is laid out otherwise than OpenGL
/// lays it out (a uniform block by std140, a storage buffer by std430),
/// said by its variable's name and the member; `None` where none is. (A
/// block's own size is not compared: no block is an element of an array,
/// and a storage buffer's is its runtime array's.)
pub(crate) fn module_differs(module: &Module) -> Option<String> {
    module.global_variables.iter().find_map(|(_, g)| {
        let rules = match g.space {
            naga::AddressSpace::Uniform => Rules::Std140,
            naga::AddressSpace::Storage { .. } => Rules::Std430,
            _ => return None,
        };
        let path = g.name.as_deref().unwrap_or("?");
        match &module.types[g.ty].inner {
            TypeInner::Struct { members, .. } => members_differ(module, members, rules, path),
            _ => differs(module, g.ty, rules, path),
        }
    })
}

impl crate::Program {
    /// Where one of the program's blocks is laid out otherwise than
    /// OpenGL lays it out: a uniform block by std140, a storage buffer by
    /// std430 (see the module's text). `None` where every one is laid out
    /// as OpenGL lays it out, as [`crate::Program::split_uniforms`] makes
    /// the blocks it makes, and refuses a program otherwise.
    pub fn opengl_layout_differs(&self) -> Option<String> {
        module_differs(&self.module)
    }
}

#[cfg(test)]
mod tests {
    use naga::Scalar;

    use super::*;

    fn insert(m: &mut Module, inner: TypeInner) -> Handle<Type> {
        m.types.insert(Type { name: None, inner }, Span::UNDEFINED)
    }

    fn member(name: &str, ty: Handle<Type>, offset: u32) -> StructMember {
        StructMember { name: Some(name.into()), ty, binding: None, offset }
    }

    /// The rules' sizes and alignments, as the GLSL specification's
    /// examples give them.
    #[test]
    fn the_rules_lay_out_as_glsl_does() {
        let mut m = Module::default();
        let f = insert(&mut m, TypeInner::Scalar(Scalar::F32));
        let v2 = insert(&mut m, TypeInner::Vector { size: VectorSize::Bi, scalar: Scalar::F32 });
        let v3 = insert(&mut m, TypeInner::Vector { size: VectorSize::Tri, scalar: Scalar::F32 });
        let m2 = insert(
            &mut m,
            TypeInner::Matrix {
                columns: VectorSize::Bi,
                rows: VectorSize::Bi,
                scalar: Scalar::F32,
            },
        );
        let m3 = insert(
            &mut m,
            TypeInner::Matrix {
                columns: VectorSize::Tri,
                rows: VectorSize::Tri,
                scalar: Scalar::F32,
            },
        );
        let n4 = ArraySize::Constant(std::num::NonZeroU32::new(4).unwrap());
        let floats = insert(&mut m, TypeInner::Array { base: f, size: n4, stride: 16 });
        let s = insert(
            &mut m,
            TypeInner::Struct { members: vec![member("a", f, 0), member("b", v2, 8)], span: 16 },
        );
        use Rules::*;
        for (ty, std140, std430) in [
            (f, (4, 4), (4, 4)),
            (v2, (8, 8), (8, 8)),
            (v3, (16, 12), (16, 12)),
            (m2, (16, 32), (8, 16)),
            (m3, (16, 48), (16, 48)),
            (floats, (16, 64), (4, 16)),
            (s, (16, 16), (8, 16)),
        ] {
            assert_eq!(of(&m, ty, Std140), Some(std140), "{:?}", m.types[ty].inner);
            assert_eq!(of(&m, ty, Std430), Some(std430), "{:?}", m.types[ty].inner);
        }
    }

    /// The members the trial change left out: an array of scalars or of
    /// `vec2`s, a struct nested in an element, and an element of scalars
    /// alone, each as glslang lays it out (std140), are told apart from
    /// std430 and laid out by it.
    #[test]
    fn std140_members_are_laid_out_anew_by_std430() {
        let mut m = Module::default();
        let f = insert(&mut m, TypeInner::Scalar(Scalar::F32));
        let i = insert(&mut m, TypeInner::Scalar(Scalar::I32));
        let v2 = insert(&mut m, TypeInner::Vector { size: VectorSize::Bi, scalar: Scalar::F32 });
        let v4 = insert(&mut m, TypeInner::Vector { size: VectorSize::Quad, scalar: Scalar::F32 });
        let n3 = ArraySize::Constant(std::num::NonZeroU32::new(3).unwrap());
        let floats = insert(&mut m, TypeInner::Array { base: f, size: n3, stride: 16 });
        let pairs = insert(&mut m, TypeInner::Array { base: v2, size: n3, stride: 16 });
        let vec4s = insert(&mut m, TypeInner::Array { base: v4, size: n3, stride: 16 });
        let inner = insert(
            &mut m,
            TypeInner::Struct { members: vec![member("x", f, 0), member("y", i, 4)], span: 16 },
        );
        let lone = insert(&mut m, TypeInner::Struct { members: vec![member("n", i, 0)], span: 16 });
        let element = insert(
            &mut m,
            TypeInner::Struct {
                members: vec![
                    member("n", i, 0),
                    member("floats", floats, 16),
                    member("pairs", pairs, 64),
                    member("inner", inner, 112),
                ],
                span: 128,
            },
        );
        use Rules::*;
        // (Both rules alike: nothing to do.)
        assert_eq!(differs(&m, vec4s, Std430, "v"), None);
        assert_eq!(relayout(&mut m, vec4s, Std430), vec4s);
        // (One int: 16 bytes by std140, 4 by std430.)
        assert_eq!(differs(&m, lone, Std140, "e"), None);
        assert_eq!(differs(&m, lone, Std430, "e").unwrap(), "e: 16 bytes, 4 by Std430");
        let why = differs(&m, element, Std430, "e").unwrap();
        assert_eq!(why, "e.floats: at byte 16, 4 by Std430");
        assert_eq!(differs(&m, element, Std140, "e"), None);
        let new = relayout(&mut m, element, Std430);
        assert_eq!(differs(&m, new, Std430, "e"), None);
        let TypeInner::Struct { members, span } = &m.types[new].inner else { panic!() };
        let at: Vec<u32> = members.iter().map(|m| m.offset).collect();
        // (n 4, floats 3 × 4, pairs 3 × 8 aligned to 8, inner 8 aligned to 4.)
        assert_eq!(at, [0, 4, 16, 40]);
        assert_eq!(*span, 48);
        let stride = |ty| match m.types[ty].inner {
            TypeInner::Array { stride, .. } => stride,
            _ => 0,
        };
        assert_eq!((stride(members[1].ty), stride(members[2].ty)), (4, 8));
        assert!(matches!(m.types[members[3].ty].inner, TypeInner::Struct { span: 8, .. }));
    }
}
