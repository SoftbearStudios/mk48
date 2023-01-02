use crate::Camera3d;
use glam::{vec2, Vec2};
use renderer::{Layer, RenderLayer, Renderer, Shader, Texture, TextureType, TriangleBuffer};

/// A [`Layer`] that renders a skybox. Must be the first layer for now.
pub struct SkyboxLayer {
    buffer: TriangleBuffer<Vec2>,
    cube_map: Texture,
    shader: Shader,
}

impl SkyboxLayer {
    /// Creates a new [`SkyboxLayer`] from a `cube_map` ([`Texture::typ`] == [`TextureType::Cube`]).
    pub fn with_cube_map(renderer: &Renderer, cube_map: Texture) -> Self {
        assert_eq!(
            cube_map.typ(),
            TextureType::Cube,
            "texture must be a cube map"
        );

        // Create a buffer that has 1 triangle covering the whole screen.
        let mut buffer = TriangleBuffer::new(renderer);
        buffer.buffer(
            renderer,
            &[vec2(-1.0, 3.0), vec2(-1.0, -1.0), vec2(3.0, -1.0)],
            &[],
        );

        let shader = renderer.create_shader(
            include_str!("shaders/skybox.vert"),
            include_str!("shaders/skybox.frag"),
        );

        Self {
            buffer,
            cube_map,
            shader,
        }
    }
}

impl Layer for SkyboxLayer {}

impl RenderLayer<&Camera3d> for SkyboxLayer {
    fn render(&mut self, renderer: &Renderer, params: &Camera3d) {
        if let Some(shader) = self.shader.bind(renderer) {
            // TODO depth <= and render last.
            renderer.set_depth_test(false);

            let mut view_matrix = params.view_matrix;
            let v = view_matrix.as_mut();
            v[12] = 0.0;
            v[13] = 0.0;
            v[14] = 0.0;
            let matrix = (params.projection_matrix * view_matrix).inverse();

            shader.uniform("uMatrix", &matrix);
            shader.uniform("uSampler", &self.cube_map);

            self.buffer.bind(renderer).draw();

            renderer.set_depth_test(true);
        }
    }
}
