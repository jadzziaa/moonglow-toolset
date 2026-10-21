//! A program's resources in naga's IR: bindings renumbered into one group
//! and described by name, which is how Moonglow fills them (the uniform
//! contract in `docs/shaders-proposal.md`).

use naga::{AddressSpace, Binding, ImageDimension, Module, ResourceBinding, TypeInner};

/// What a value is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    Float,
    Int,
    Uint,
    Bool,
    /// `vec2` to `vec4` (and their `ivec`, `uvec`, `bvec`).
    Vector(u8, Scalar),
    /// `matCxR`: columns, rows, of floats.
    Matrix(u8, u8),
    /// A struct, or something else a uniform isn't.
    Other,
}

/// A scalar's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scalar {
    Float,
    Int,
    Uint,
    Bool,
}

/// A member of a uniform block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// As the shader names it: `m_mvp`, `lightPosition`, `fSwaySpeed`.
    pub name: String,
    /// In bytes from the start of the block.
    pub offset: u32,
    /// The value's type (an array's element's).
    pub ty: ValueType,
    /// An array's length (`lightPosition[MAX_NUM_LIGHTS]`); `None` for one value.
    pub count: Option<u32>,
    /// An array's stride in bytes; for one value, its size.
    pub stride: u32,
    /// How many bytes it takes in all.
    pub size: u32,
}

/// Which of a program's uniforms a block holds: all of them (glslang's
/// one block, as compiled), or, split by [`crate::Program::split_uniforms`],
/// a draw's own, the frame's, or the bones'.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Part {
    #[default]
    Whole,
    Draw,
    Frame,
    Bones,
}

/// A uniform block: glslang's `gl_DefaultUniformBlock` holds the loose
/// uniforms, one block per program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniformBlock {
    /// The block's type name (`gl_DefaultUniformBlock`).
    pub name: String,
    pub part: Part,
    /// Its bind group ([`Reflection::group`], unless moved by
    /// [`crate::Program::place_block`]).
    pub group: u32,
    pub binding: u32,
    /// Its size in bytes (`span`), which arrays set (`m_bones`: 4 KB).
    pub size: u32,
    pub members: Vec<Member>,
}

impl UniformBlock {
    pub fn member(&self, name: &str) -> Option<&Member> {
        self.members.iter().find(|m| m.name == name)
    }
}

/// A texture the shader samples, and the sampler split from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    /// The shader's name for it: `texUnit0`, `texUnitEnvCube`.
    pub name: String,
    pub dim: ImageDimension,
    /// The texture's binding.
    pub binding: u32,
    /// Its sampler's binding (the next one).
    pub sampler: Option<u32>,
}

/// A stage input or output with a location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Varying {
    /// `vPos`, `vNormal`, `vTcIn`; a varying's name in both stages.
    pub name: String,
    pub location: u32,
    pub ty: ValueType,
}

/// What a program reads and writes, by name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reflection {
    /// The bind group of every resource (0, unless [`crate::Program::set_group`]).
    pub group: u32,
    pub uniforms: Vec<UniformBlock>,
    pub textures: Vec<Texture>,
    /// Vertex attributes for a vertex shader, varyings for a fragment one.
    pub inputs: Vec<Varying>,
    /// Varyings for a vertex shader, the color for a fragment one.
    pub outputs: Vec<Varying>,
}

impl Reflection {
    /// A uniform by name, in whichever block has it.
    pub fn uniform(&self, name: &str) -> Option<(&UniformBlock, &Member)> {
        self.uniforms.iter().find_map(|b| b.member(name).map(|m| (b, m)))
    }

    pub fn texture(&self, name: &str) -> Option<&Texture> {
        self.textures.iter().find(|t| t.name == name)
    }
}

fn value_type(module: &Module, inner: &TypeInner) -> ValueType {
    let scalar = |s: naga::Scalar| match s.kind {
        naga::ScalarKind::Sint => Scalar::Int,
        naga::ScalarKind::Uint => Scalar::Uint,
        naga::ScalarKind::Bool => Scalar::Bool,
        _ => Scalar::Float,
    };
    match inner {
        TypeInner::Scalar(s) => match scalar(*s) {
            Scalar::Float => ValueType::Float,
            Scalar::Int => ValueType::Int,
            Scalar::Uint => ValueType::Uint,
            Scalar::Bool => ValueType::Bool,
        },
        TypeInner::Vector { size, scalar: s } => ValueType::Vector(*size as u8, scalar(*s)),
        TypeInner::Matrix { columns, rows, .. } => ValueType::Matrix(*columns as u8, *rows as u8),
        TypeInner::Array { base, .. } => value_type(module, &module.types[*base].inner),
        _ => ValueType::Other,
    }
}

pub(crate) fn member(module: &Module, m: &naga::StructMember) -> Member {
    let inner = &module.types[m.ty].inner;
    let size = inner.size(module.to_ctx());
    let (count, stride) = match inner {
        TypeInner::Array { size: naga::ArraySize::Constant(n), stride, .. } => {
            (Some(n.get()), *stride)
        }
        _ => (None, size),
    };
    Member {
        name: m.name.clone().unwrap_or_default(),
        offset: m.offset,
        ty: value_type(module, inner),
        count,
        stride,
        size,
    }
}

/// Puts every resource in group 0, uniform blocks first, then each
/// texture with its sampler after it (`texUnit0_image`, `texUnit0_sampler`
/// from the split), then anything else; and describes them.
pub(crate) fn bind(module: &mut Module, io: &[crate::spirv::Io]) -> Reflection {
    let mut r = Reflection::default();
    let mut next = 0u32;
    let mut take = |module: &mut Module, h: naga::Handle<naga::GlobalVariable>| {
        let b = next;
        module.global_variables[h].binding = Some(ResourceBinding { group: 0, binding: b });
        next += 1;
        b
    };
    let handles: Vec<_> = module.global_variables.iter().map(|(h, _)| h).collect();
    let is_resource = |g: &naga::GlobalVariable| {
        matches!(
            g.space,
            AddressSpace::Uniform | AddressSpace::Storage { .. } | AddressSpace::Handle
        )
    };
    let mut done = std::collections::HashSet::new();
    for &h in &handles {
        let g = &module.global_variables[h];
        // (The spare blocks a split fills are bound after the rest.)
        let spare = g.name.as_deref().is_some_and(|n| crate::spirv::SPARE_BLOCKS.contains(&n));
        if g.space != AddressSpace::Uniform || spare {
            continue;
        }
        let ty = g.ty;
        let binding = take(module, h);
        done.insert(h);
        let t = &module.types[ty];
        let (members, size) = match &t.inner {
            TypeInner::Struct { members, span } => {
                (members.iter().map(|m| member(module, m)).collect(), *span)
            }
            inner => (Vec::new(), inner.size(module.to_ctx())),
        };
        r.uniforms.push(UniformBlock {
            name: t.name.clone().unwrap_or_default(),
            part: Part::Whole,
            group: 0,
            binding,
            size,
            members,
        });
    }
    for &h in &handles {
        let g = &module.global_variables[h];
        let TypeInner::Image { dim, .. } = module.types[g.ty].inner else { continue };
        let name = g.name.clone().unwrap_or_default();
        let base = name.strip_suffix("_image").unwrap_or(&name).to_string();
        let binding = take(module, h);
        done.insert(h);
        let pair = format!("{base}_sampler");
        let sampler = handles.iter().copied().find(|&s| {
            let g = &module.global_variables[s];
            !done.contains(&s)
                && g.name.as_deref() == Some(pair.as_str())
                && matches!(module.types[g.ty].inner, TypeInner::Sampler { .. })
        });
        let sampler = sampler.map(|s| {
            done.insert(s);
            take(module, s)
        });
        r.textures.push(Texture { name: base, dim, binding, sampler });
    }
    for &h in &handles {
        if !done.contains(&h) && is_resource(&module.global_variables[h]) {
            take(module, h);
        }
    }
    if let Some(ep) = module.entry_points.first() {
        let varying = |name: &Option<String>, ty, binding: &Option<Binding>| match binding {
            Some(Binding::Location { location, .. }) => Some(Varying {
                name: name.clone().unwrap_or_default(),
                location: *location,
                ty: value_type(module, &module.types[ty].inner),
            }),
            _ => None,
        };
        let add = |ty, name: &Option<String>, binding: &Option<Binding>, out: &mut Vec<Varying>| {
            match &module.types[ty].inner {
                TypeInner::Struct { members, .. } if binding.is_none() => {
                    out.extend(members.iter().filter_map(|m| varying(&m.name, m.ty, &m.binding)));
                }
                _ => out.extend(varying(name, ty, binding)),
            }
        };
        for a in &ep.function.arguments {
            add(a.ty, &a.name, &a.binding, &mut r.inputs);
        }
        if let Some(res) = &ep.function.result {
            add(res.ty, &None, &res.binding, &mut r.outputs);
        }
        // The names as the SPIR-V gave them (an output's isn't in the IR).
        for (list, output) in [(&mut r.inputs, false), (&mut r.outputs, true)] {
            for v in list.iter_mut() {
                let named = io.iter().find(|i| i.output == output && i.location == v.location);
                if let Some(i) = named.filter(|i| !i.name.is_empty()) {
                    v.name = i.name.clone();
                }
            }
            list.sort_by_key(|v| v.location);
        }
    }
    r
}
