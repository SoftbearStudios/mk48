// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::buffer::{MeshBuffer, RenderBuffer};
use crate::renderer::framebuffer::Framebuffer;
use crate::renderer::renderer::{Layer, Renderer};
use crate::renderer::shader::{Shader, ShaderBinding};
use crate::renderer::vertex::PosUv;
use glam::{uvec2, vec2, vec4, IVec2, UVec2, Vec2};
use std::ops::Range;
use web_sys::WebGlRenderingContext as Gl;

/// Invalidates part of the frame cache (ex. background texture changed).
/// TODO allow invalidating only part of the screen.
#[derive(Debug)]
pub enum Invalidation {
    /// Redraw the whole buffer.
    All,
    /// Redraw only the part of the buffer covered by some rectangles.
    /// Coordinates are in world space.
    Rects(Vec<(Vec2, Vec2)>),
}

/// Any extra data that is necessary to render the background.
pub trait BackgroundContext {
    fn create_shader(&self, _: &mut Renderer) -> Shader;
    fn prepare(&mut self, renderer: &Renderer, shader: &mut ShaderBinding);
    fn frame_cache_enabled(&self) -> bool {
        false
    }
    fn take_invalidation(&mut self) -> Option<Invalidation> {
        None
    }
}

/// A whole-screen layer.
pub struct BackgroundLayer<C: BackgroundContext> {
    pub context: C,
    shader: Shader,
    frame_cache: Option<FrameCache>,
}

impl<C: BackgroundContext> BackgroundLayer<C> {
    /// Shader must take uCamera and uMiddle_uDerivative uniforms.
    pub fn new(renderer: &mut Renderer, context: C) -> Self {
        let shader = context.create_shader(renderer);

        let gl: &Gl = &renderer.gl;
        let oes_vao = &renderer.oes_vao;
        renderer.background_buffer.get_or_insert_with(|| {
            let mut buffer = RenderBuffer::new(gl, oes_vao);
            buffer_viewport(gl, &mut buffer);
            buffer
        });

        let frame_cache = context
            .frame_cache_enabled()
            .then(|| FrameCache::new(renderer));

        Self {
            context,
            shader,
            frame_cache,
        }
    }
}

/// Buffers a single triangle that covers the whole viewport.
fn buffer_viewport(gl: &Gl, buffer: &mut RenderBuffer<PosUv>) {
    buffer_viewport_with_uv(gl, buffer, |uv| uv);
}

/// Buffers a single triangle that covers the whole viewport.
/// Calls the provided function to get the uv from the screen space position.
fn buffer_viewport_with_uv<F: FnMut(Vec2) -> Vec2>(
    gl: &Gl,
    buffer: &mut RenderBuffer<PosUv>,
    mut f: F,
) {
    buffer.buffer(
        gl,
        &[vec2(-1.0, 3.0), vec2(-1.0, -1.0), vec2(3.0, -1.0)].map(|pos| PosUv { pos, uv: f(pos) }),
        &[],
    );
}

impl<C: BackgroundContext> Layer for BackgroundLayer<C> {
    /// The background shader will be applied to the entire screen.
    /// Prepare can bind any other necessary uniforms.
    fn render(&mut self, renderer: &Renderer) {
        let mut wrote_none: bool = false;

        // Write to frame cache or main screen.
        'write: {
            if let Some(mut shader) = renderer.bind_shader(&self.shader) {
                let mut buffer = renderer.background_buffer.as_ref().unwrap();
                let mut camera = &renderer.camera;

                let _fbb = if let Some(frame_cache) = &mut self.frame_cache {
                    // Update the frame cached (resize texture and compute read/write buffers).
                    let invalidation = self.context.take_invalidation();
                    frame_cache.update(
                        renderer,
                        renderer.aligned_camera.delta_pixels,
                        invalidation,
                    );

                    // Not drawing anything (camera didn't move).
                    if !frame_cache.write_some {
                        wrote_none = true;
                        break 'write;
                    }

                    buffer = &frame_cache.write_buffer;
                    camera = &renderer.aligned_camera;

                    Some(frame_cache.scrolled_frame.bind(renderer))
                } else {
                    None
                };

                // Set uniforms.
                let matrix = &camera.camera_matrix;
                let viewport_meters = (matrix.transform_point2(Vec2::splat(1.0))
                    - matrix.transform_point2(Vec2::splat(-1.0)))
                .abs();

                let middle = camera.center();
                let derivative = viewport_meters / camera.viewport.as_vec2();

                shader.uniform_matrix3f("uCamera", matrix);

                // Pack middle and derivative.
                shader.uniform4f(
                    "uMiddle_uDerivative",
                    vec4(middle.x, middle.y, derivative.x, derivative.y),
                );

                // Set custom uniforms.
                self.context.prepare(renderer, &mut shader);

                buffer
                    .bind(&renderer.gl, &renderer.oes_vao)
                    .draw(Gl::TRIANGLES);
            }
        }

        // Read from frame cache and render to main screen.
        if let Some(frame_cache) = &mut self.frame_cache {
            if let Some(shader) = renderer.bind_shader(&frame_cache.shader) {
                // Don't need to copy if original wasn't modified.
                if !wrote_none {
                    shader.uniform_texture("uSampler", frame_cache.scrolled_frame.as_texture(), 0);

                    let fb_binding = frame_cache.linear_frame.bind(renderer);

                    // Read buffer is used to read from frame cache.
                    frame_cache
                        .read_buffer
                        .bind(&renderer.gl, &renderer.oes_vao)
                        .draw(Gl::TRIANGLES);

                    drop(fb_binding);
                }

                shader.uniform_texture("uSampler", frame_cache.linear_frame.as_texture(), 0);

                // Move uv by subpixel difference to remove visible pixel snapping.
                let subpixel = renderer.aligned_camera.subpixel_uv_diff(&renderer.camera);

                let buffer = &mut frame_cache.subpixel_buffer;
                buffer_viewport_with_uv(&renderer.gl, buffer, |uv| uv - subpixel);
                buffer
                    .bind(&renderer.gl, &renderer.oes_vao)
                    .draw(Gl::TRIANGLES);
            }
        }
    }
}

struct FrameCache {
    shader: Shader,
    linear_frame: Framebuffer,
    scrolled_frame: Framebuffer,
    scroll: UVec2,
    read_buffer: RenderBuffer<PosUv>,
    subpixel_buffer: RenderBuffer<PosUv>,
    write_buffer: RenderBuffer<PosUv>,
    write_some: bool,
}

impl FrameCache {
    fn new(renderer: &Renderer) -> Self {
        // Compile framebuffer shader (alternative to WebGL2's blitFramebuffer).
        let shader = renderer.create_shader(
            include_str!("shaders/framebuffer.vert"),
            include_str!("shaders/framebuffer.frag"),
        );

        let linear_frame = Framebuffer::new(renderer, true);
        let scrolled_frame = Framebuffer::new(renderer, false);
        let scroll = UVec2::ZERO;

        let read_buffer = RenderBuffer::new(&renderer.gl, &renderer.oes_vao);
        let subpixel_buffer = RenderBuffer::new(&renderer.gl, &renderer.oes_vao);
        let write_buffer = RenderBuffer::new(&renderer.gl, &renderer.oes_vao);
        let write_some = false;

        Self {
            shader,
            linear_frame,
            scrolled_frame,
            scroll,
            read_buffer,
            subpixel_buffer,
            write_buffer,
            write_some,
        }
    }

    fn update(
        &mut self,
        renderer: &Renderer,
        maybe_delta: Option<IVec2>,
        invalidation: Option<Invalidation>,
    ) {
        let viewport = renderer.canvas_size();
        self.scrolled_frame.set_viewport(&renderer.gl, viewport);
        self.linear_frame.set_viewport(&renderer.gl, viewport);

        const DEBUG: bool = false;

        // Validate that a delta can be made.
        let delta = {
            if let Some(delta) = maybe_delta {
                if delta.abs().cmpge(viewport.as_ivec2()).any() {
                    if DEBUG {
                        crate::console_log!("invalidated: out of range");
                    }
                    None
                } else if let Some(inv) = invalidation {
                    if DEBUG {
                        crate::console_log!("invalidated: {:?}", inv);
                    }
                    match inv {
                        Invalidation::All => None,
                        Invalidation::Rects(rects) => Some((delta, rects)),
                    }
                } else {
                    Some((delta, vec![]))
                }
            } else {
                if DEBUG {
                    crate::console_log!("invalidated: zoom");
                }
                None
            }
        };

        let delta_pos = delta.as_ref().map(|(p, _)| *p);
        let delta_rects = delta.map(|(_, r)| r).unwrap_or_default();
        let draw = delta_pos.unwrap_or(viewport.as_ivec2());

        // Scroll by delta pos (more efficient than copying).
        self.scroll = if let Some(d) = delta_pos {
            snapping_add_uvec2(self.scroll, d, viewport)
        } else {
            UVec2::ZERO
        };

        let scroll = self.scroll;
        let gl = &renderer.gl;

        // Apply scroll when writing to buffer.
        // Also only write to the fraction of the buffer that came on screen.
        // Up to 2 viewport quads are converted into up to 8 scrolled quads.
        let x1 = 0u32.saturating_add_signed(-draw.x);
        let x2 = viewport.x - (draw.x).max(0) as u32;

        let y1 = 0u32.saturating_add_signed(-draw.y);
        let y2 = viewport.y - (draw.y).max(0) as u32;

        // For debugging purposes.
        let mut pixels_drawn = 0;

        self.write_some = Self::buffer_scrolled_rects(
            gl,
            &mut self.write_buffer,
            Self::iter_scrolled_rects(uvec2(x1, 0), uvec2(viewport.x, y1), scroll, viewport)
                .chain(
                    Self::iter_scrolled_rects(uvec2(x2, y1), viewport, scroll, viewport).chain(
                        Self::iter_scrolled_rects(
                            uvec2(0, y2),
                            uvec2(x2, viewport.y),
                            scroll,
                            viewport,
                        )
                        .chain(Self::iter_scrolled_rects(
                            UVec2::ZERO,
                            uvec2(x1, y2),
                            scroll,
                            viewport,
                        ))
                        .chain(delta_rects.into_iter().flat_map(|rect| {
                            let (start, end) = rect;
                            let matrix = &renderer.aligned_camera.view_matrix;

                            let v = viewport.as_vec2();
                            let multiplier = v * 0.5;

                            let s = ((matrix.transform_point2(start) + 1.0) * multiplier)
                                .clamp(Vec2::ZERO, v)
                                .floor()
                                .as_uvec2();
                            let e = ((matrix.transform_point2(end) + 1.0) * multiplier)
                                .clamp(Vec2::ZERO, v)
                                .ceil()
                                .as_uvec2();

                            Self::iter_scrolled_rects(s, e, scroll, viewport)
                        })),
                    ),
                )
                .inspect(|(rect, _)| {
                    let diff = rect.end - rect.start;
                    pixels_drawn += diff.x * diff.y
                }),
            viewport,
        );

        if DEBUG && pixels_drawn > 0 {
            crate::console_log!("drawing {} pixels", pixels_drawn);
        }

        // Undo scroll when reading from buffer.
        let inv_scroll = viewport - scroll;
        Self::buffer_scrolled_rects(
            gl,
            &mut self.read_buffer,
            Self::iter_scrolled_rects(UVec2::ZERO, viewport, inv_scroll, viewport),
            viewport,
        );
    }

    /// Trianglulates and buffers an iterator of rects to a render buffer.
    /// The generated triangles aren't necessarily in the same order as the iterator.
    /// Returns if any triangles were generated.
    fn buffer_scrolled_rects(
        gl: &Gl,
        buffer: &mut RenderBuffer<PosUv>,
        rects: impl Iterator<Item = (Rect, Rect)>,
        viewport: UVec2,
    ) -> bool {
        let viewport_rect = Rect {
            start: UVec2::ZERO,
            end: viewport,
        };
        let multiplier = viewport.as_vec2().recip() * 2.0;

        let mut mesh = MeshBuffer::new();
        let mut single_triangle = false;

        mesh.vertices.extend(
            rects
                .map(|(rect, scrolled)| {
                    if rect == viewport_rect && scrolled == viewport_rect {
                        single_triangle = true;
                        None
                    } else {
                        let Rect { start, end } = rect;
                        let Rect {
                            start: start2,
                            end: end2,
                        } = scrolled;

                        Some(
                            [
                                (start, start2),
                                (uvec2(end.x, start.y), uvec2(end2.x, start2.y)),
                                (uvec2(start.x, end.y), uvec2(start2.x, end2.y)),
                                (end, end2),
                            ]
                            .map(|(point, snapped)| {
                                // Map 0..viewport to -1.0..1.0
                                let pos = point.as_vec2() * multiplier - 1.0;
                                let uv = snapped.as_vec2() * multiplier - 1.0;

                                PosUv { pos, uv }
                            }),
                        )
                    }
                })
                .flatten()
                .flatten(),
        );

        if single_triangle {
            buffer_viewport(gl, buffer);
            true
        } else {
            if mesh.vertices.is_empty() {
                false
            } else {
                mesh.push_default_quads();
                buffer.buffer_mesh(gl, &mesh);
                true
            }
        }
    }

    // calculates the up to 4 rectangle pairs required to scroll a single rectangle.
    // draw_start and draw_end must be less than or equal to viewport.
    fn iter_scrolled_rects(
        rect_start: UVec2,
        rect_end: UVec2,
        scroll: UVec2,
        viewport: UVec2,
    ) -> impl Iterator<Item = (Rect, Rect)> {
        // assert that the draw rect is within the viewport.
        assert!(rect_start.cmple(viewport).all());
        assert!(rect_end.cmple(viewport).all());

        // Returns a quad if it is greater than zero area.
        let ranges_to_rect = |x: &Range<u32>, y: &Range<u32>| {
            if x.is_empty() || y.is_empty() {
                None
            } else {
                let start = uvec2(x.start, y.start);
                let start2 = snapping_sub_uvec2(start, scroll, viewport, false);
                let end = uvec2(x.end, y.end);
                let end2 = snapping_sub_uvec2(end, scroll, viewport, true);
                Some((
                    Rect { start, end },
                    Rect {
                        start: start2,
                        end: end2,
                    },
                ))
            }
        };

        let inv_scroll = viewport - scroll;

        // Ranges used to make the 4 quads.
        let x_range = rect_start.x.checked_sub(inv_scroll.x).unwrap_or(0)
            ..scroll.x.saturating_sub(viewport.x - rect_end.x);
        let x_range2 = (scroll.x + rect_start.x).min(viewport.x)
            ..(viewport.x - inv_scroll.x.checked_sub(rect_end.x).unwrap_or(0));
        let y_range = rect_start.y.checked_sub(inv_scroll.y).unwrap_or(0)
            ..scroll.y.saturating_sub(viewport.y - rect_end.y);
        let y_range2 = (scroll.y + rect_start.y).min(viewport.y)
            ..(viewport.y - inv_scroll.y.checked_sub(rect_end.y).unwrap_or(0));

        // Iter the 4 rects (zero area rects are elided).
        std::array::IntoIter::new([
            ranges_to_rect(&x_range, &y_range),
            ranges_to_rect(&x_range2, &y_range),
            ranges_to_rect(&x_range, &y_range2),
            ranges_to_rect(&x_range2, &y_range2),
        ])
        .flatten()
    }
}

// An axis alligned integer rectangle.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct Rect {
    start: UVec2,
    end: UVec2,
}

/// Vector snapping add.
fn snapping_add_uvec2(a: UVec2, b: IVec2, snap: UVec2) -> UVec2 {
    uvec2(
        snapping_add(a.x, b.x, snap.x),
        snapping_add(a.y, b.y, snap.y),
    )
}

/// Like wrapping add signed but snaps to a given snap value.
/// Adds b to a while snapping to snap.
fn snapping_add(a: u32, b: i32, snap: u32) -> u32 {
    assert!(a <= snap);
    assert!(b.abs() as u32 <= snap);

    if b >= 0 {
        let r = a + b as u32;
        if r > snap {
            r - snap
        } else {
            r
        }
    } else {
        snapping_sub(a, b.abs() as u32, snap, false)
    }
}

// Vector snapping sub.
// end is the same for both vectors.
fn snapping_sub_uvec2(a: UVec2, b: UVec2, snap: UVec2, end: bool) -> UVec2 {
    uvec2(
        snapping_sub(a.x, b.x, snap.x, end),
        snapping_sub(a.y, b.y, snap.y, end),
    )
}

/// Like wrapping sub but overflows to a given snap value.
/// Subtracts b from a while snapping to snap.
/// If end 0 wraps to snap.
fn snapping_sub(a: u32, b: u32, snap: u32, end: bool) -> u32 {
    assert!(a <= snap);
    assert!(b <= snap);

    let s = if a >= b { a - b } else { a + snap - b };

    if end && s == 0 {
        snap
    } else {
        s
    }
}
