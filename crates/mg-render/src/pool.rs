//! Meshes' vertices and indices in a few large buffers, so that a frame's
//! draws bind them once and each draw names its own by where it starts.
//!
//! A large area's picture is tens of thousands of draws of different
//! meshes. With a vertex and an index buffer of each mesh's own, every draw
//! bound both, and wgpu's work for binding them (the buffers looked up,
//! their use tracked and merged, their initialized ranges checked, and the
//! driver's binding) was a third of the frame's time on the processor.
//! Meshes now lie in chunks: a draw binds a chunk's buffers where the draw
//! before was of another chunk, and draws its indices from its first, its
//! vertices counted from its base.
//!
//! A chunk is made when the meshes don't fit those made before, as large
//! as those together (up to the device's largest buffer): a model viewer's
//! few meshes take a small one, a large area's millions of vertices a few
//! (draws of other chunks in a row bind theirs, so few and large is what
//! counts). A mesh larger than that has a chunk of its own. What a dropped
//! mesh held is free again for the meshes that come after, and a chunk
//! whose meshes are all dropped is dropped with them.
//! Skinned meshes lie in chunks of their own, with a third buffer of their
//! bones and weights at the same places as their vertices.

use std::sync::{Arc, Mutex, Weak};

use crate::Gpu;
use crate::model::{SkinVertex, Vertex};

/// The first chunk's vertices; each chunk made after has as many as those
/// before it together, up to [`MAX_VERTICES`] and the device's largest
/// buffer.
const FIRST_VERTICES: u32 = 1 << 14;
const MAX_VERTICES: u32 = 1 << 22;
/// A chunk's indices for each of its vertices.
const INDICES_PER_VERTEX: u32 = 4;

/// Ranges free in a chunk's buffer (start, length), in order and apart.
#[derive(Debug, Default)]
struct Free(Vec<(u32, u32)>);

impl Free {
    /// Takes `n` from the first free range that holds them: where they
    /// begin.
    fn take(&mut self, n: u32) -> Option<u32> {
        let at = self.0.iter().position(|&(_, len)| len >= n)?;
        let (start, len) = self.0[at];
        if len == n {
            self.0.remove(at);
        } else {
            self.0[at] = (start + n, len - n);
        }
        Some(start)
    }

    /// Gives `n` from `start` back, joined to the free ranges either side.
    fn give(&mut self, start: u32, n: u32) {
        if n == 0 {
            return;
        }
        let at = self.0.partition_point(|&(s, _)| s < start);
        let joins_before = at > 0 && {
            let (s, len) = self.0[at - 1];
            s + len == start
        };
        let joins_after = self.0.get(at).is_some_and(|&(s, _)| start + n == s);
        match (joins_before, joins_after) {
            (true, true) => {
                let (_, after) = self.0.remove(at);
                self.0[at - 1].1 += n + after;
            }
            (true, false) => self.0[at - 1].1 += n,
            (false, true) => self.0[at] = (start, self.0[at].1 + n),
            (false, false) => self.0.insert(at, (start, n)),
        }
    }
}

/// A chunk's buffers and what is free in them.
#[derive(Debug)]
pub(crate) struct Chunk {
    /// A number of its own among the chunks made: a draw binds the chunk's
    /// buffers where the draw before was of another.
    pub(crate) id: u64,
    pub(crate) vertices: wgpu::Buffer,
    pub(crate) indices: wgpu::Buffer,
    /// A skinned chunk's bones and weights, at its vertices' places.
    pub(crate) skin: Option<wgpu::Buffer>,
    /// How many vertices it holds.
    size: u32,
    /// Free vertices and indices.
    free: Mutex<(Free, Free)>,
}

/// Where a mesh lies in its chunk; given back as it is dropped.
#[derive(Debug)]
pub(crate) struct Slot {
    pub(crate) chunk: Arc<Chunk>,
    pub(crate) base_vertex: u32,
    pub(crate) vertex_count: u32,
    pub(crate) first_index: u32,
    pub(crate) index_count: u32,
}

impl Drop for Slot {
    fn drop(&mut self) {
        let mut free = self.chunk.free.lock().unwrap_or_else(|e| e.into_inner());
        free.0.give(self.base_vertex, self.vertex_count);
        free.1.give(self.first_index, self.index_count);
    }
}

/// The chunks of a device's meshes.
#[derive(Debug, Default)]
pub(crate) struct MeshPool {
    chunks: Mutex<Chunks>,
}

#[derive(Debug, Default)]
struct Chunks {
    /// Each chunk while any of its meshes is kept.
    live: Vec<Weak<Chunk>>,
    made: u64,
}

fn buffer(gpu: &Gpu, label: &str, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(16),
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

impl MeshPool {
    /// Puts a mesh's vertices and indices (and a skinned one's bones and
    /// weights, one for each vertex) into a chunk with room for them.
    pub(crate) fn put(
        &self,
        gpu: &Gpu,
        vertices: &[Vertex],
        indices: &[u32],
        skin: Option<&[SkinVertex]>,
    ) -> Slot {
        let (nv, ni) = (vertices.len() as u32, indices.len() as u32);
        let skinned = skin.is_some();
        let mut chunks = self.chunks.lock().unwrap_or_else(|e| e.into_inner());
        chunks.live.retain(|c| c.strong_count() > 0);
        let mut found = None;
        for chunk in chunks.live.iter().filter_map(Weak::upgrade) {
            if chunk.skin.is_some() != skinned {
                continue;
            }
            let mut free = chunk.free.lock().unwrap_or_else(|e| e.into_inner());
            let Some(v) = free.0.take(nv) else { continue };
            match free.1.take(ni) {
                Some(i) => {
                    drop(free);
                    found = Some((chunk, v, i));
                    break;
                }
                None => free.0.give(v, nv),
            }
        }
        let (chunk, base_vertex, first_index) = match found {
            Some(f) => f,
            None => {
                // (As large as those of its kind together, within the
                // device's largest buffer; a chunk of its own for a mesh
                // too large for one.)
                let stride = std::mem::size_of::<Vertex>() as u64;
                let largest = gpu.device.limits().max_buffer_size;
                let fits = (largest / stride).min(largest / 4 / u64::from(INDICES_PER_VERTEX));
                let fits = u32::try_from(fits).unwrap_or(u32::MAX).min(MAX_VERTICES);
                let before: u32 = chunks
                    .live
                    .iter()
                    .filter_map(Weak::upgrade)
                    .filter(|c| c.skin.is_some() == skinned)
                    .map(|c| c.size)
                    .fold(0, u32::saturating_add);
                let size = before.clamp(FIRST_VERTICES, fits.max(FIRST_VERTICES)).max(nv);
                let index_size = size.saturating_mul(INDICES_PER_VERTEX).max(ni);
                chunks.made += 1;
                if std::env::var_os("MG_POOL_DEBUG").is_some() {
                    eprintln!(
                        "pool: chunk {} of {size} vertices, {index_size} indices, skinned {skinned}",
                        chunks.made
                    );
                }
                let id = chunks.made;
                let label = format!("meshes {id}");
                let chunk = Arc::new(Chunk {
                    id,
                    vertices: buffer(
                        gpu,
                        &label,
                        u64::from(size) * stride,
                        wgpu::BufferUsages::VERTEX,
                    ),
                    indices: buffer(
                        gpu,
                        &label,
                        u64::from(index_size) * 4,
                        wgpu::BufferUsages::INDEX,
                    ),
                    skin: skinned.then(|| {
                        let stride = std::mem::size_of::<SkinVertex>() as u64;
                        buffer(gpu, &label, u64::from(size) * stride, wgpu::BufferUsages::VERTEX)
                    }),
                    size,
                    free: Mutex::new((
                        Free(vec![(nv, size - nv)]),
                        Free(vec![(ni, index_size - ni)]),
                    )),
                });
                chunks.live.push(Arc::downgrade(&chunk));
                (chunk, 0, 0)
            }
        };
        drop(chunks);
        let write = |buffer: &wgpu::Buffer, at: u64, bytes: &[u8]| {
            if !bytes.is_empty() {
                gpu.queue.write_buffer(buffer, at, bytes);
            }
        };
        let vertex_size = std::mem::size_of::<Vertex>() as u64;
        write(
            &chunk.vertices,
            u64::from(base_vertex) * vertex_size,
            bytemuck::cast_slice(vertices),
        );
        write(&chunk.indices, u64::from(first_index) * 4, bytemuck::cast_slice(indices));
        if let (Some(buffer), Some(skin)) = (&chunk.skin, skin) {
            let skin_size = std::mem::size_of::<SkinVertex>() as u64;
            write(buffer, u64::from(base_vertex) * skin_size, bytemuck::cast_slice(skin));
        }
        Slot { chunk, base_vertex, vertex_count: nv, first_index, index_count: ni }
    }
}

#[cfg(test)]
mod tests {
    use super::Free;

    #[test]
    fn free_ranges_are_taken_first_fit_and_given_back_joined() {
        let mut f = Free(vec![(0, 100)]);
        assert_eq!(f.take(30), Some(0));
        assert_eq!(f.take(30), Some(30));
        assert_eq!(f.take(50), None);
        assert_eq!(f.take(40), Some(60));
        assert_eq!(f.0, []);
        f.give(30, 30);
        assert_eq!(f.0, [(30, 30)]);
        f.give(0, 30);
        assert_eq!(f.0, [(0, 60)]);
        f.give(80, 20);
        assert_eq!(f.0, [(0, 60), (80, 20)]);
        f.give(60, 20);
        assert_eq!(f.0, [(0, 100)]);
        // Nothing to give.
        f.give(5, 0);
        assert_eq!(f.0, [(0, 100)]);
    }
}
