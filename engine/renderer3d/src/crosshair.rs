use crate::Camera3d;
use glam::{vec2, Vec2};
use renderer::{gray, DefaultRender, Layer, RenderLayer, Renderer, Shader, TriangleBuffer};

const THICKNESS: f32 = 0.0066;
const RADIUS: f32 = 0.025;

/// A [`Layer`] that renders a crosshair in the center of the screen.
pub struct CrosshairLayer {
    buffer: TriangleBuffer<Vec2>,
    shader: Shader,
}

impl DefaultRender for CrosshairLayer {
    fn new(renderer: &Renderer) -> Self {
        let mut buffer = TriangleBuffer::new(renderer);

        let t = (THICKNESS / RADIUS) * 0.5;
        let vertices = [
            vec2(-t, -t),
            vec2(t, -t),
            vec2(-t, t),
            vec2(t, t),
            vec2(-1.0, -t),
            vec2(-1.0, t),
            vec2(-t, -1.0),
            vec2(t, -1.0),
            vec2(1.0, -t),
            vec2(1.0, t),
            vec2(-t, 1.0),
            vec2(t, 1.0),
        ];
        let indices = [
            0, 1, 2, 3, 2, 1, 4, 0, 2, 5, 4, 3, 4, 0, 2, 5, 4, 3, 6, 1, 0, 1, 6, 7, 8, 3, 1, 3, 8,
            9, 2, 3, 10, 3, 11, 10,
        ];
        buffer.buffer(renderer, &vertices, &indices);

        let shader = renderer.create_shader(
            include_str!("shaders/crosshair.vert"),
            include_str!("shaders/crosshair.frag"),
        );

        Self { buffer, shader }
    }
}

impl Layer for CrosshairLayer {}

impl RenderLayer<&Camera3d> for CrosshairLayer {
    fn render(&mut self, renderer: &Renderer, _: &Camera3d) {
        if let Some(shader) = self.shader.bind(renderer) {
            let scale = Vec2::splat(RADIUS) * vec2(renderer.aspect_ratio().recip(), 1.0);
            let color = gray(200);

            shader.uniform("uScale", scale);
            shader.uniform("uColor", color);

            self.buffer.bind(renderer).draw();
        }
    }
}
