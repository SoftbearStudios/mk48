use glam::*;
use js_hooks::console_log;
use renderer::{gray_a, rgba, Layer};
use renderer2d::{Camera2d, GraphicLayer, Renderer2d, TextLayer};

#[derive(Layer)]
#[layer(Camera2d)]
struct MyLayer {
    graphics: GraphicLayer,
    text: TextLayer,
}

fn main() {
    // Required to get stack traces in WASM.
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    // Create renderer with antialiasing.
    let antialias = true;
    let mut renderer = Renderer2d::new(antialias).expect("no webgl");

    // Set background color to gray.
    let gray = gray_a(50, 255);
    renderer.set_background_color(gray);

    // Create camera at (0, 0)
    let center = vec2(0.0, 0.0);
    let zoom = 2.5;
    renderer.camera.update(center, zoom, renderer.canvas_size());

    // Create our layer.
    let mut layer = MyLayer {
        graphics: GraphicLayer::new(&renderer),
        text: TextLayer::new(&renderer),
    };

    // Prepare with time set to 0.
    let time_seconds = 0.0;
    renderer.pre_prepare(&mut layer, time_seconds);

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

    // Render everything that was drawn.
    renderer.render(&mut layer);

    console_log!("Done!")
}
