use glam::*;
use js_hooks::console_log;
use renderer::{gray_a, rgba, DefaultRender, Layer, RenderChain};
use renderer2d::{Camera2d, GraphicLayer, TextLayer};

#[derive(Layer)]
#[render(&Camera2d)]
struct MyLayer {
    graphics: GraphicLayer,
    text: TextLayer,
}

fn main() {
    // Required to get stack traces in WASM.
    #[cfg(target_family = "wasm")]
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    // Create renderer with antialiasing.
    let antialias = true;
    let mut render_chain = RenderChain::new(antialias, |r| {
        // Set background color to gray.
        let gray = gray_a(50, 255);
        r.set_background_color(gray);

        // Create our layer.
        MyLayer {
            graphics: GraphicLayer::new(r),
            text: TextLayer::new(r),
        }
    })
    .expect("no webgl");

    // Prepare with time set to 0.
    let time_seconds = 0.0;
    let mut frame = render_chain.begin(time_seconds);
    let (renderer, layer) = frame.draw();

    // Draw a red line from.
    let start = vec2(-0.9, -0.4);
    let end = vec2(0.9, -0.4);
    let thickness = 0.2;
    let s = rgba(255, 100, 10, 255);
    let e = rgba(10, 255, 100, 255);
    layer
        .graphics
        .draw_rounded_line_gradient(start, end, thickness, s, e, false);

    // Draw a gray circle.
    let center = vec2(0.0, 0.0);
    let radius = 1.2;
    let thickness = 0.1;
    let color = gray_a(100, 255);
    layer.graphics.draw_circle(center, radius, thickness, color);

    // Draw some transparent yellow text.
    let center = vec2(0.0, 0.5);
    let scale = 1.0;
    let color = rgba(255, 255, 0, 100);
    layer.text.draw("| |", center, scale, color);

    let mut camera = Camera2d::default();
    // Create camera at (0, 0)
    let center = vec2(0.0, 0.0);
    let zoom = 2.5;
    camera.update(center, zoom, renderer.canvas_size());

    // Render everything that was drawn.
    frame.end(&camera);

    console_log!("Done!")
}
