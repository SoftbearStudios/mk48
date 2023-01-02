// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::*;
use renderer::{derive_vertex, MeshBuilder, Vertex};
use std::mem::size_of;

// Re-export.
pub use engine_macros::include_ply;

/// A static 3D model that has vertices and indices. Its vertices contain positions, normals, uvs,
/// and colors.
#[derive(Debug)]
pub struct Model {
    /// Untyped float slice of vertices (so alignment is 4).
    pub vertices: &'static [f32],
    /// [`prim@u16`] indices.
    pub indices: &'static [u16],
    /// If the [`Model`] has [`Vec3`] normals.
    pub normals: bool,
    /// If the [`Model`] has [`Vec2`] uvs.
    pub uvs: bool,
    /// If the [`Model`] has [`Vec4`] colors,
    pub colors: bool,
}

impl Model {
    /// Allocates a model as a [`MeshBuilder`].
    pub fn to_builder<V: Vertex>(&self) -> MeshBuilder<V> {
        assert_eq!(
            self.vertex_floats() * std::mem::size_of::<f32>(),
            size_of::<V>(),
            "vertex size mismatch: {:?}",
            self
        );

        let mut builder = MeshBuilder::new();
        let vertices: &'static [V] = bytemuck::cast_slice(self.vertices);
        builder.vertices = vertices.to_owned();
        builder.indices = self.indices.to_owned();
        builder
    }

    /// Returns the size of each [`Vertex`] in floats.
    pub(crate) fn vertex_floats(&self) -> usize {
        // positions always Vec3.
        let mut floats = 3;
        if self.normals {
            floats += 3;
        }
        if self.uvs {
            floats += 2;
        }
        if self.colors {
            floats += 4;
        }
        floats
    }
}

derive_vertex!(
    /// Basic [`Vertex`] for testing.
    #[doc(hidden)]
    #[derive(Debug)]
    pub struct TestVertex {
        pos: Vec3,
        normal: Vec3,
        uv: Vec2,
    }
);

/// Basic model made with [`TextVertex`] vertices.
#[doc(hidden)]
pub const TEST_MODEL: Model = include_ply!("models/test.ply");

#[cfg(test)]
mod tests {
    use super::*;
    use renderer::MeshBuilder;

    #[test]
    fn model_to_builder() {
        let model = TEST_MODEL;
        println!("{:?}", model);
        let builder: MeshBuilder<TestVertex, _> = model.to_builder();
        println!("{:?}", builder);
    }
}
