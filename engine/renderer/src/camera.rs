// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::Renderer;
use crate::shader::ShaderBinding;

/// A 2D or 3D view into world space.
pub trait Camera: Default {
    /// Sets `uniform mat3 uView;` for `Camera2D` and `uniform mat4 uViewProjection;` for
    /// `Camera3D`.
    fn uniform_matrix(&self, shader: &ShaderBinding);
    /// TODO remove this hack for Camera3d and replace with DepthLayer.
    #[doc(hidden)]
    fn init_render(renderer: &Renderer<Self>) {
        let _ = renderer;
    }
}
