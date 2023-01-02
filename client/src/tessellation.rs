use glam::{uvec2, UVec2, Vec2};
use renderer::{
    DefaultRender, Layer, MeshBuilder, RenderLayer, Renderer, ShaderBinding, TriangleBuffer,
};

pub struct TessellationLayer {
    buffer: TriangleBuffer<Vec2, u32>,
    previous_size: UVec2,
}

impl DefaultRender for TessellationLayer {
    fn new(renderer: &Renderer) -> Self {
        Self {
            buffer: TriangleBuffer::new(renderer),
            previous_size: UVec2::ZERO,
        }
    }
}

impl TessellationLayer {
    fn buffer(&mut self, renderer: &Renderer, dim: UVec2) -> &TriangleBuffer<Vec2, u32> {
        // Don't recreate the same plane.
        if self.previous_size != dim {
            self.previous_size = dim;
            let scale = 1.0 / dim.as_vec2();

            let mut mesh = MeshBuilder::new();
            mesh.vertices.extend(
                (0..(dim.x + 1))
                    .flat_map(|x| (0..(dim.y + 1)).map(move |y| uvec2(x, y).as_vec2() * scale)),
            );
            mesh.indices.extend((0..dim.x).flat_map(|x| {
                (0..dim.y).flat_map(move |y| {
                    let v00 = y + x * (dim.y + 1);
                    let v10 = y + (x + 1) * (dim.y + 1);
                    let v01 = (y + 1) + x * (dim.y + 1);
                    let v11 = (y + 1) + (x + 1) * (dim.y + 1);
                    [v00, v10, v11, v11, v01, v00]
                })
            }));
            self.buffer.buffer_mesh(renderer, &mesh);
        }

        &self.buffer
    }
}

impl Layer for TessellationLayer {}

impl RenderLayer<(&ShaderBinding<'_>, UVec2)> for TessellationLayer {
    fn render(&mut self, renderer: &Renderer, (_shader, dim): (&ShaderBinding, UVec2)) {
        self.buffer(renderer, dim).bind(renderer).draw();
    }
}
