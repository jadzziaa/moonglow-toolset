//! Compiled (binary) models: a 32-bit memory dump. After a 12-byte header
//! (zero, raw data offset, raw data size), "pointers" in the model section
//! are offsets from byte 12 and vertex data offsets are from the raw data.
//! A node's type is only in its flags (+0x6C); function pointers, parent
//! pointers and padding hold garbage in EE-compiled files and are ignored.
//! Every offset is bounds-checked. Layout: `docs/research/notes_models.md`
//! §B, from nwnmdlcomp's headers, checked on every compiled model in the
//! game.

use std::collections::HashSet;

use crate::ctrl::{self, flags};
use crate::{
    AabbEntry, AnimMesh, AnimMeshSets, AnimNode, Animation, Classification, Controller, Dangly,
    Emitter, Face, Light, MdlError, Mesh, MeshExtra, Model, Node, NodeKind, Reference, Skin, Vec2,
    Vec3,
};

/// The most nodes a model (or animation) may have; guards against loops.
const MAX_NODES: usize = 100_000;

struct Bin<'a> {
    data: &'a [u8],
    /// Model data base (file offset 12).
    m: usize,
    /// Raw data base.
    r: usize,
}

fn err(what: impl Into<String>) -> MdlError {
    MdlError::Binary(what.into())
}

impl<'a> Bin<'a> {
    fn bytes(&self, at: usize, n: usize) -> Result<&'a [u8], MdlError> {
        at.checked_add(n)
            .and_then(|end| self.data.get(at..end))
            .ok_or_else(|| err(format!("{n} bytes at {at:#x} are past the end")))
    }

    fn u8(&self, off: usize) -> Result<u8, MdlError> {
        Ok(self.bytes(self.m + off, 1)?[0])
    }
    fn u16(&self, off: usize) -> Result<u16, MdlError> {
        let b = self.bytes(self.m + off, 2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&self, off: usize) -> Result<u32, MdlError> {
        let b = self.bytes(self.m + off, 4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f32(&self, off: usize) -> Result<f32, MdlError> {
        Ok(f32::from_bits(self.u32(off)?))
    }
    fn vec3(&self, off: usize) -> Result<Vec3, MdlError> {
        Ok([self.f32(off)?, self.f32(off + 4)?, self.f32(off + 8)?])
    }

    /// A NUL-terminated name in a fixed field (garbage after the NUL).
    fn name(&self, off: usize, n: usize) -> Result<String, MdlError> {
        let b = self.bytes(self.m + off, n)?;
        let end = b.iter().position(|&c| c == 0).unwrap_or(n);
        Ok(b[..end].iter().map(|&c| c as char).collect())
    }

    /// A texture or model name: lower case, `None` for empty or `NULL`.
    fn resname(&self, off: usize, n: usize) -> Result<Option<String>, MdlError> {
        let s = self.name(off, n)?.to_ascii_lowercase();
        Ok((!s.is_empty() && s != "null").then_some(s))
    }

    /// A NUL-terminated name at an absolute offset (lower case, at most 64
    /// bytes).
    fn cstr_at(&self, abs: usize) -> Result<String, MdlError> {
        let tail =
            self.data.get(abs..).ok_or_else(|| err(format!("name at {abs:#x} is past the end")))?;
        let tail = &tail[..tail.len().min(64)];
        let end = tail.iter().position(|&c| c == 0).unwrap_or(tail.len());
        Ok(tail[..end].iter().map(|&c| (c as char).to_ascii_lowercase()).collect())
    }

    /// An array header: (offset, count), the count checked against the data
    /// for elements of `size` bytes.
    fn array(&self, off: usize, size: usize) -> Result<(usize, usize), MdlError> {
        let offset = self.u32(off)? as usize;
        let count = self.u32(off + 4)? as usize;
        if count == 0 {
            return Ok((0, 0));
        }
        self.bytes(self.m + offset, count.saturating_mul(size.max(1)))?;
        Ok((offset, count))
    }

    fn f32s(&self, base: usize, count: usize) -> Result<Vec<f32>, MdlError> {
        let b = self.bytes(base, count.checked_mul(4).ok_or_else(|| err("count overflow"))?)?;
        Ok(b.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect())
    }

    /// A raw-data stream, `None` for the 0xFFFFFFFF null.
    fn raw(&self, off: usize) -> Result<Option<usize>, MdlError> {
        let p = self.u32(off)?;
        Ok((p != u32::MAX).then_some(self.r + p as usize))
    }

    fn vec3s_at(&self, abs: usize, count: usize) -> Result<Vec<Vec3>, MdlError> {
        let f = self.f32s(abs, count * 3)?;
        Ok(f.as_chunks::<3>().0.to_vec())
    }

    fn vec2s_at(&self, abs: usize, count: usize) -> Result<Vec<Vec2>, MdlError> {
        let f = self.f32s(abs, count * 2)?;
        Ok(f.as_chunks::<2>().0.to_vec())
    }

    fn controllers(&self, node: usize, node_flags: u32) -> Result<Vec<Controller>, MdlError> {
        let (keys, nkeys) = self.array(node + 0x54, 12)?;
        let (data_off, ndata) = self.array(node + 0x60, 4)?;
        let data = if ndata > 0 { self.f32s(self.m + data_off, ndata)? } else { Vec::new() };
        let mut out = Vec::with_capacity(nkeys);
        for i in 0..nkeys {
            let k = keys + i * 12;
            let id = self.u32(k)?;
            let rows = self.u16(k + 4)? as usize;
            let time_at = self.u16(k + 6)? as usize;
            let value_at = self.u16(k + 8)? as usize;
            let cols = self.u8(k + 10)? as i8;
            // −1 means no values; 0x10 marks Bézier keys, whose keys hold
            // their value and two handles (as the game's compiler writes
            // them; no game model has any).
            let bezier = cols >= 0 && cols & 0x10 != 0;
            let columns = if cols < 0 { 0 } else { (cols & 0x0F) as usize };
            let name = ctrl::name(node_flags, id);
            let times = data.get(time_at..time_at + rows).map(<[f32]>::to_vec).unwrap_or_default();
            let stride = if bezier { columns * 3 } else { columns };
            // A declared column count past the data (one light's colour
            // claims 4): fall back to the controller's own.
            let (columns, stride) = match ctrl::columns(node_flags, &name) {
                Some(c) if !bezier && value_at + rows * columns > data.len() => (c, c),
                _ => (columns, stride),
            };
            let keyed = data.get(value_at..value_at + times.len() * stride);
            let (times, values, handles) = match keyed {
                Some(v) if bezier && columns > 0 => {
                    let mut values = Vec::with_capacity(times.len() * columns);
                    let mut handles = Vec::with_capacity(times.len() * columns * 2);
                    for key in v.chunks_exact(stride) {
                        values.extend_from_slice(&key[..columns]);
                        handles.extend_from_slice(&key[columns..]);
                    }
                    (times, values, handles)
                }
                Some(v) => (times, v.to_vec(), Vec::new()),
                None => (Vec::new(), Vec::new(), Vec::new()),
            };
            // A repeated controller (fx_flame01's light has two shadow
            // radii) replaces the earlier one, as a repeated ASCII keyword
            // does.
            out.retain(|c: &Controller| c.name != name);
            out.push(Controller { name, times, values, columns, handles });
        }
        Ok(out)
    }

    fn children(&self, node: usize) -> Result<Vec<usize>, MdlError> {
        let (at, n) = self.array(node + 0x48, 4)?;
        (0..n).map(|i| Ok(self.u32(at + i * 4)? as usize)).collect()
    }

    /// Walks a node tree in pre-order: (offset, parent index).
    fn walk(&self, root: usize) -> Result<Vec<(usize, Option<usize>)>, MdlError> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![(root, None)];
        while let Some((off, parent)) = stack.pop() {
            if !seen.insert(off) || out.len() >= MAX_NODES {
                return Err(err(format!("node tree loops or is too large at {off:#x}")));
            }
            let index = out.len();
            out.push((off, parent));
            for c in self.children(off)?.into_iter().rev() {
                stack.push((c, Some(index)));
            }
        }
        Ok(out)
    }

    fn mesh(&self, n: usize, node_flags: u32) -> Result<Mesh, MdlError> {
        let mut m = Mesh {
            diffuse: self.vec3(n + 0xAC)?,
            ambient: self.vec3(n + 0xB8)?,
            specular: self.vec3(n + 0xC4)?,
            shininess: self.f32(n + 0xD0)?,
            shadow: self.u32(n + 0xD4)? != 0,
            beaming: self.u32(n + 0xD8)? != 0,
            render: self.u32(n + 0xDC)? != 0,
            transparency_hint: self.u32(n + 0xE0)?,
            tilefade: self.u32(n + 0x1E8)?,
            rotate_texture: self.u8(n + 0x265)? != 0,
            // The game's compiler: 1 `None`, 2 `NormalAndSpecMapped`, 3
            // `NormalTangents` (other values: none given).
            renderhint: match self.u32(n + 0xE4)? {
                1 => Some("none".into()),
                2 => Some("normalandspecmapped".into()),
                3 => Some("normaltangents".into()),
                _ => None,
            },
            ..Default::default()
        };
        // The fourth slot holds the `materialname`: the game's compiler
        // writes it there (and drops `texture3`).
        for (i, t) in m.textures.iter_mut().take(3).enumerate() {
            *t = self.resname(n + 0xE8 + 64 * i, 64)?;
        }
        m.material = self.resname(n + 0xE8 + 64 * 3, 64)?;
        let count = self.u16(n + 0x230)? as usize;
        if let Some(at) = self.raw(n + 0x22C)?.filter(|_| count > 0) {
            m.vertices = self.vec3s_at(at, count)?;
        }
        let count = m.vertices.len();
        if count > 0 {
            for (i, uv) in m.uvs.iter_mut().enumerate() {
                if let Some(at) = self.raw(n + 0x234 + 4 * i)? {
                    *uv = self.vec2s_at(at, count)?;
                }
            }
            if let Some(at) = self.raw(n + 0x244)? {
                m.normals = self.vec3s_at(at, count)?;
            }
            if let Some(at) = self.raw(n + 0x248)? {
                let b = self.bytes(at, count * 4)?;
                m.colors = b.as_chunks::<4>().0.to_vec();
            }
            // With a render hint, the game's compiler stores tangents and
            // the bitangents' signs in what 1.69 files use for water's
            // "bump map animation" streams 4 and 6.
            if m.renderhint.as_deref().is_some_and(|h| h != "none")
                && let (Some(t), Some(h)) = (self.raw(n + 0x258)?, self.raw(n + 0x260)?)
            {
                let tangents = self.vec3s_at(t, count)?;
                let signs = self.f32s(h, count)?;
                m.tangents =
                    tangents.iter().zip(&signs).map(|(t, &s)| [t[0], t[1], t[2], s]).collect();
            }
        }
        let (faces, nfaces) = self.array(n + 0x78, 32)?;
        m.faces = (0..nfaces)
            .map(|i| {
                let f = faces + i * 32;
                let v = [self.u16(f + 0x1A)?, self.u16(f + 0x1C)?, self.u16(f + 0x1E)?];
                Ok(Face { vertices: v.map(u32::from), material: self.u32(f + 0x10)? })
            })
            .collect::<Result<_, MdlError>>()?;
        m.source = (0..count as u32).collect();
        m.source_uv = m.source.clone();
        m.extra = if node_flags & flags::SKIN != 0 {
            MeshExtra::Skin(self.skin(n, count)?)
        } else if node_flags & flags::DANGLY != 0 {
            let (at, k) = self.array(n + 0x270, 4)?;
            MeshExtra::Dangly(Dangly {
                constraints: if k > 0 { self.f32s(self.m + at, k)? } else { Vec::new() },
                displacement: self.f32(n + 0x27C)?,
                tightness: self.f32(n + 0x280)?,
                period: self.f32(n + 0x284)?,
            })
        } else if node_flags & flags::ANIM != 0 {
            MeshExtra::Anim(AnimMesh { sample_period: self.f32(n + 0x270)? })
        } else if node_flags & flags::AABB != 0 {
            MeshExtra::Aabb(self.aabb(self.u32(n + 0x270)? as usize)?)
        } else {
            MeshExtra::None
        };
        Ok(m)
    }

    fn skin(&self, n: usize, count: usize) -> Result<Skin, MdlError> {
        let mut skin = Skin::default();
        let map_at = self.u32(n + 0x284)? as usize;
        let map_len = self.u32(n + 0x288)? as usize;
        let map = self.bytes(self.m + map_at, map_len.saturating_mul(2))?;
        let mut mapped = Vec::new();
        for (node, c) in map.as_chunks::<2>().0.iter().enumerate() {
            let bone = i16::from_le_bytes(*c);
            if bone >= 0 {
                let b = bone as usize;
                if skin.bones.len() <= b {
                    skin.bones.resize(b + 1, 0);
                    mapped.resize(b + 1, false);
                }
                skin.bones[b] = node;
                mapped[b] = true;
            }
        }
        if let (Some(w), Some(r)) = (self.raw(n + 0x27C)?, self.raw(n + 0x280)?) {
            let weights = self.f32s(w, count * 4)?;
            let refs = self.bytes(r, count * 8)?;
            skin.weights = (0..count)
                .map(|v| {
                    let mut out = [(0u16, 0.0f32); 4];
                    for (j, o) in out.iter_mut().enumerate() {
                        let at = (v * 4 + j) * 2;
                        let bone = i16::from_le_bytes([refs[at], refs[at + 1]]);
                        // A few skins (c_wings2) weight bones that map to no
                        // node: those vertices keep their rest pose.
                        if bone >= 0 && mapped.get(bone as usize).copied().unwrap_or(false) {
                            *o = (bone as u16, weights[v * 4 + j]);
                        }
                    }
                    out
                })
                .collect();
        }
        let (q, nq) = self.array(n + 0x28C, 16)?;
        let (t, nt) = self.array(n + 0x298, 12)?;
        for i in 0..nq.min(nt) {
            // Stored w, x, y, z.
            let w = self.f32(q + i * 16)?;
            let v = self.vec3(q + i * 16 + 4)?;
            skin.inverse_bind.push(([v[0], v[1], v[2], w], self.vec3(t + i * 12)?));
        }
        Ok(skin)
    }

    fn aabb(&self, root: usize) -> Result<Vec<AabbEntry>, MdlError> {
        let mut out = Vec::new();
        let mut stack = vec![root];
        let mut seen = HashSet::new();
        while let Some(e) = stack.pop() {
            if e == 0 && !out.is_empty() || !seen.insert(e) || out.len() > MAX_NODES * 4 {
                continue;
            }
            out.push(AabbEntry {
                min: self.vec3(e)?,
                max: self.vec3(e + 12)?,
                face: self.u32(e + 0x20)? as i32,
            });
            let (left, right) = (self.u32(e + 0x18)? as usize, self.u32(e + 0x1C)? as usize);
            if right != 0 {
                stack.push(right);
            }
            if left != 0 {
                stack.push(left);
            }
        }
        Ok(out)
    }

    fn light(&self, n: usize) -> Result<Light, MdlError> {
        let floats = |off| -> Result<Vec<f32>, MdlError> {
            let (at, k) = self.array(off, 4)?;
            if k == 0 { Ok(Vec::new()) } else { self.f32s(self.m + at, k) }
        };
        let (shifts, ns) = self.array(n + 0x98, 12)?;
        let (names, nn) = self.array(n + 0xA4, 4)?;
        Ok(Light {
            flare_radius: self.f32(n + 0x70)?,
            flare_sizes: floats(n + 0x80)?,
            flare_positions: floats(n + 0x8C)?,
            flare_color_shifts: (0..ns)
                .map(|i| self.vec3(shifts + i * 12))
                .collect::<Result<_, _>>()?,
            flare_textures: (0..nn)
                .map(|i| self.cstr_at(self.m + self.u32(names + i * 4)? as usize))
                .collect::<Result<_, MdlError>>()?,
            priority: self.u32(n + 0xB0)?,
            ambient_only: self.u32(n + 0xB4)? != 0,
            dynamic_type: self.u32(n + 0xB8)?,
            affect_dynamic: self.u32(n + 0xBC)? != 0,
            shadow: self.u32(n + 0xC0)? != 0,
            generate_flare: self.u32(n + 0xC4)? != 0,
            fading: self.u32(n + 0xC8)? != 0,
        })
    }

    fn emitter(&self, n: usize) -> Result<Emitter, MdlError> {
        Ok(Emitter {
            deadspace: self.f32(n + 0x70)?,
            blast_radius: self.f32(n + 0x74)?,
            blast_length: self.f32(n + 0x78)?,
            xgrid: self.u32(n + 0x7C)?,
            ygrid: self.u32(n + 0x80)?,
            spawntype: self.u32(n + 0x84)?,
            update: self.name(n + 0x88, 32)?,
            render: self.name(n + 0xA8, 32)?,
            blend: self.name(n + 0xC8, 32)?,
            texture: self.resname(n + 0xE8, 64)?,
            chunk: self.resname(n + 0x128, 16)?,
            two_sided: self.u32(n + 0x138)? != 0,
            looping: self.u32(n + 0x13C)? != 0,
            render_order: u32::from(self.u16(n + 0x140)?),
            flags: self.u32(n + 0x144)?,
        })
    }

    fn kind(&self, n: usize, node_flags: u32) -> Result<NodeKind, MdlError> {
        Ok(if node_flags & flags::MESH != 0 {
            NodeKind::Mesh(Box::new(self.mesh(n, node_flags)?))
        } else if node_flags & flags::LIGHT != 0 {
            NodeKind::Light(self.light(n)?)
        } else if node_flags & flags::EMITTER != 0 {
            NodeKind::Emitter(self.emitter(n)?)
        } else if node_flags & flags::REFERENCE != 0 {
            NodeKind::Reference(Reference {
                model: self.resname(n + 0x70, 64)?,
                reattachable: self.u32(n + 0xB0)? != 0,
            })
        } else if node_flags & flags::CAMERA != 0 {
            NodeKind::Camera
        } else {
            NodeKind::Dummy
        })
    }

    fn animation(
        &self,
        a: usize,
        vertex_counts: &[(String, usize)],
    ) -> Result<Animation, MdlError> {
        let mut anim = Animation {
            name: self.name(a + 0x08, 64)?,
            length: self.f32(a + 0x70)?,
            transtime: self.f32(a + 0x74)?,
            animroot: self.name(a + 0x78, 64)?,
            ..Default::default()
        };
        let (ev, nev) = self.array(a + 0xB8, 36)?;
        for i in 0..nev {
            anim.events.push((self.f32(ev + i * 36)?, self.name(ev + i * 36 + 4, 32)?));
        }
        let root = self.u32(a + 0x48)? as usize;
        if root == 0 {
            return Ok(anim);
        }
        for (off, parent) in self.walk(root)? {
            let node_flags = self.u32(off + 0x6C)?;
            let name = self.name(off + 0x20, 32)?;
            let mut node = AnimNode {
                controllers: self.controllers(off, node_flags)?,
                parent,
                ..Default::default()
            };
            if node_flags & flags::ANIM != 0 && node_flags & flags::MESH != 0 {
                let vsets = self.u32(off + 0x2A0)? as usize;
                let tsets = self.u32(off + 0x2A4)? as usize;
                let own = self.u16(off + 0x230)? as usize;
                let count = if own > 0 {
                    own
                } else {
                    vertex_counts
                        .iter()
                        .find(|(n, _)| n.eq_ignore_ascii_case(&name))
                        .map_or(0, |(_, c)| *c)
                };
                if (vsets > 0 || tsets > 0) && count > 0 {
                    let verts = self.u32(off + 0x298)? as usize;
                    let tverts = self.u32(off + 0x29C)? as usize;
                    let v = if vsets > 0 {
                        self.vec3s_at(self.m + verts, count * vsets)?
                    } else {
                        Vec::new()
                    };
                    let t = if tsets > 0 {
                        self.vec2s_at(self.m + tverts, count * tsets)?
                    } else {
                        Vec::new()
                    };
                    // Stored vertex-major: index = vertex × sets + set.
                    node.anim_mesh = Some(AnimMeshSets {
                        sample_period: self.f32(off + 0x270)?,
                        vertex_sets: (0..vsets)
                            .map(|s| (0..count).map(|i| v[i * vsets + s]).collect())
                            .collect(),
                        uv_sets: (0..tsets)
                            .map(|s| (0..count).map(|i| t[i * tsets + s]).collect())
                            .collect(),
                    });
                }
            }
            node.name = name;
            anim.nodes.push(node);
        }
        Ok(anim)
    }
}

/// Reads a compiled model.
pub fn read(data: &[u8]) -> Result<Model, MdlError> {
    if !crate::is_binary(data) {
        return Err(err("not a compiled model"));
    }
    let raw_offset = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let b = Bin { data, m: 12, r: 12usize.saturating_add(raw_offset) };
    let supermodel = b.resname(0xA8, 64)?;
    let mut model = Model {
        name: b.name(0x08, 64)?,
        supermodel,
        classification: Classification::from_byte(b.u8(0x72)?),
        ignore_fog: b.u8(0x73)? == 0,
        animation_scale: b.f32(0xA4)?,
        ..Default::default()
    };
    let root = b.u32(0x48)? as usize;
    if root == 0 {
        return Err(MdlError::Empty);
    }
    for (off, parent) in b.walk(root)? {
        let node_flags = b.u32(off + 0x6C)?;
        let mut node = Node::new(&b.name(off + 0x20, 32)?, b.kind(off, node_flags)?);
        node.parent = parent;
        node.inherit_color = b.u32(off + 0x18)? != 0;
        for c in b.controllers(off, node_flags)? {
            let row = c.row(0);
            match c.name.as_str() {
                "position" if row.len() == 3 => node.position = [row[0], row[1], row[2]],
                "orientation" if row.len() == 4 => {
                    node.orientation = [row[0], row[1], row[2], row[3]];
                }
                "scale" if row.len() == 1 => node.scale = row[0],
                _ => node.controllers.push(c),
            }
        }
        if let Some(p) = parent {
            let index = model.nodes.len();
            model.nodes[p].children.push(index);
        }
        model.nodes.push(node);
    }
    let vertex_counts: Vec<(String, usize)> = model
        .nodes
        .iter()
        .filter_map(|n| n.mesh().map(|m| (n.name.clone(), m.vertices.len())))
        .collect();
    let (anims, count) = b.array(0x78, 4)?;
    for i in 0..count {
        let at = b.u32(anims + i * 4)? as usize;
        model.animations.push(b.animation(at, &vertex_counts)?);
    }
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A model section laid out by offset (little-endian).
    #[derive(Default)]
    struct Out(Vec<u8>);

    impl Out {
        fn put(&mut self, at: usize, bytes: &[u8]) {
            if self.0.len() < at + bytes.len() {
                self.0.resize(at + bytes.len(), 0);
            }
            self.0[at..at + bytes.len()].copy_from_slice(bytes);
        }
        fn u32(&mut self, at: usize, v: u32) {
            self.put(at, &v.to_le_bytes());
        }
        fn u16(&mut self, at: usize, v: u16) {
            self.put(at, &v.to_le_bytes());
        }
        fn f32(&mut self, at: usize, v: f32) {
            self.u32(at, v.to_bits());
        }
        fn floats(&mut self, at: usize, v: &[f32]) {
            for (i, x) in v.iter().enumerate() {
                self.f32(at + 4 * i, *x);
            }
        }
        fn name(&mut self, at: usize, s: &str) {
            self.put(at, s.as_bytes());
        }
    }

    /// What `nwmain compilemodel` writes for a mesh with `bitmap tex0`,
    /// `materialname mymaterial` and `renderhint NormalTangents` (the
    /// tangents it generates), and a `positionbezierkey`.
    fn compiled_by_the_game() -> Vec<u8> {
        let mut m = Out::default();
        m.name(0x08, "probe");
        m.u32(0x48, 0x100);
        m.u32(0x78, 0x500);
        m.u32(0x7C, 1);
        m.f32(0xA4, 1.0);
        // The root, with the mesh as its child.
        m.name(0x120, "probe");
        m.u32(0x16C, flags::HEADER);
        m.u32(0x148, 0x1F0);
        m.u32(0x14C, 1);
        m.u32(0x1F0, 0x200);
        let n = 0x200;
        m.name(n + 0x20, "quad");
        m.u32(n + 0x6C, flags::HEADER | flags::MESH);
        m.u32(n + 0x78, 0x480);
        m.u32(n + 0x7C, 1);
        m.u16(0x480 + 0x1C, 1);
        m.u16(0x480 + 0x1E, 2);
        m.u32(n + 0xE4, 3);
        m.name(n + 0xE8, "tex0");
        m.name(n + 0xE8 + 3 * 64, "mymaterial");
        m.u32(n + 0x22C, 0);
        m.u16(n + 0x230, 3);
        for i in 0..4 {
            m.u32(n + 0x234 + 4 * i, u32::MAX);
        }
        m.u32(n + 0x244, 36);
        m.u32(n + 0x248, u32::MAX);
        for i in 0..6 {
            m.u32(n + 0x24C + 4 * i, u32::MAX);
        }
        m.u32(n + 0x258, 72);
        m.u32(n + 0x260, 108);
        m.u32(n + 0x26C, 0);
        // The animation: a Bézier position key (flag 0x10, three columns,
        // the value and two handles).
        m.u32(0x500, 0x510);
        m.name(0x518, "go");
        m.u32(0x558, 0x600);
        m.f32(0x580, 2.0);
        m.name(0x600 + 0x20, "probe");
        m.u32(0x66C, flags::HEADER);
        m.u32(0x654, 0x700);
        m.u32(0x658, 1);
        m.u32(0x700, 8);
        m.u16(0x704, 1);
        m.u16(0x708, 1);
        m.put(0x70A, &[0x13]);
        m.u32(0x660, 0x720);
        m.u32(0x664, 10);
        m.floats(0x720, &[0.0, 1.0, 2.0, 3.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6]);
        m.put(0x7FF, &[0]);
        let mut raw = Out::default();
        raw.floats(0, &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        raw.floats(36, &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
        raw.floats(72, &[0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0]);
        raw.floats(108, &[-1.0, -1.0, -1.0]);
        let mut file = vec![0; 4];
        file.extend((m.0.len() as u32).to_le_bytes());
        file.extend((raw.0.len() as u32).to_le_bytes());
        file.extend(m.0);
        file.extend(raw.0);
        file
    }

    #[test]
    fn what_the_games_compiler_adds() {
        let model = read(&compiled_by_the_game()).unwrap();
        let mesh = model.nodes[1].mesh().unwrap();
        assert_eq!(mesh.textures[0].as_deref(), Some("tex0"));
        assert_eq!(mesh.textures[3], None);
        assert_eq!(mesh.material.as_deref(), Some("mymaterial"));
        assert_eq!(mesh.renderhint.as_deref(), Some("normaltangents"));
        assert_eq!(mesh.tangents, [[0.0, 1.0, 0.0, -1.0]; 3]);
        let key = &model.animations[0].nodes[0].controllers[0];
        assert_eq!((key.name.as_str(), key.columns), ("position", 3));
        assert_eq!(key.values, [1.0, 2.0, 3.0]);
        assert_eq!(key.handles, [0.1, 0.2, 0.3, 0.4, 0.5, 0.6]);
    }

    #[test]
    fn truncated_files_are_errors() {
        let file = compiled_by_the_game();
        for len in (0..file.len()).step_by(7) {
            let _ = read(&file[..len]);
        }
    }
}
