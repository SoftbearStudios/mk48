// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::camera_2d::Camera2d;
use crate::Renderer2d;
use glam::{uvec2, vec2, vec4, IVec2, UVec2, Vec2};
use js_hooks::console_log;
use renderer::{
    derive_vertex, Framebuffer, Layer, LayerShader, MeshBuilder, Shader, TriangleBuffer,
};
use std::ops::Range;

/// Invalidates part of the [`BackgroundLayer`]'s [`cached frame`][`BackgroundContext::cache_frame`]
/// (useful if [`BackgroundContext`]'s uniforms change).
#[derive(Debug)]
pub enum Invalidation {
    /// Redraw the whole buffer.
    All,
    /// Redraw only the part of the buffer covered by some rectangles.
    /// Coordinates are in world space.
    Rects(Vec<(Vec2, Vec2)>),
}

/// Implements [`LayerShader<Camera2d>`] and has optional methods for caching the frame.
pub trait BackgroundContext: LayerShader<Camera2d> {
    /// Returns if `true` if [`BackgroundLayer`] should cache the frame with a [`Framebuffer`].
    /// If it returns `true` [`take_invalidation`][`Self::take_invalidation`] must be implemented
    /// correctly. NOTE: Only called once in [`BackgroundLayer::new`] so shouldn't change. Only use
    /// this on expensive [`Shader`]s that don't make [`Invalidation`]s that often for performance
    /// gains.
    fn cache_frame(&self) -> bool {
        false
    }
    /// Reports which pixels were invalidated by changes to uniforms in
    /// [`prepare`][`LayerShader::prepare`]. This will be called every frame if
    /// [`cache_frame`][`Self::cache_frame`] returned `true`.
    fn take_invalidation(&mut self) -> Option<Invalidation> {
        None
    }
}

derive_vertex!(
    struct PosUv {
        pos: Vec2,
        uv: Vec2,
    }
);

/// Draws a [`Shader`] over the whole screen.
pub struct BackgroundLayer<X: BackgroundContext> {
    buffer: TriangleBuffer<PosUv, u16>,
    frame_cache: Option<FrameCache>,
    /// The [`BackgroundContext`] passed to [`new`][`Self::new`].
    pub context: X,
    shader: Shader,
    shader_loaded: bool,
}

impl<X: BackgroundContext> BackgroundLayer<X> {
    /// Shader must take uCamera and uMiddle_uDerivative uniforms.
    pub fn new(renderer: &Renderer2d, context: X) -> Self {
        let shader = context.create(renderer);
        let shader_loaded = false;

        let mut buffer = TriangleBuffer::new(renderer);
        buffer_viewport(renderer, &mut buffer);

        let frame_cache = context.cache_frame().then(|| FrameCache::new(renderer));

        Self {
            buffer,
            context,
            frame_cache,
            shader,
            shader_loaded,
        }
    }
}

/// Buffers a single triangle that covers the whole viewport.
fn buffer_viewport(renderer: &Renderer2d, buffer: &mut TriangleBuffer<PosUv>) {
    buffer_viewport_with_uv(renderer, buffer, |uv| uv);
}

/// Buffers a single triangle that covers the whole viewport.
/// Calls the provided function to get the uv from the screen space position.
fn buffer_viewport_with_uv<F: FnMut(Vec2) -> Vec2>(
    renderer: &Renderer2d,
    buffer: &mut TriangleBuffer<PosUv>,
    mut f: F,
) {
    buffer.buffer(
        renderer,
        &[vec2(-1.0, 3.0), vec2(-1.0, -1.0), vec2(3.0, -1.0)].map(|pos| PosUv { pos, uv: f(pos) }),
        &[],
    );
}

impl<X: BackgroundContext> Layer<Camera2d> for BackgroundLayer<X> {
    fn render(&mut self, renderer: &Renderer2d) {
        let mut wrote_none: bool = false;

        // Write to frame cache or main screen.
        'write: {
            if let Some(mut shader) = self.shader.bind(renderer) {
                let just_loaded = !self.shader_loaded;
                self.shader_loaded = true;

                let mut buffer = &self.buffer;
                let mut camera_matrix = &renderer.camera.camera_matrix;
                let mut middle = renderer.camera.center;

                let _fbb = if let Some(frame_cache) = &mut self.frame_cache {
                    // Update the frame cached (resize texture and compute read/write buffers).
                    let mut invalidation = self.context.take_invalidation();
                    if just_loaded {
                        invalidation = Some(Invalidation::All);
                    }

                    frame_cache.update(
                        renderer,
                        renderer.camera.aligned.delta_pixels,
                        invalidation,
                    );

                    // Not drawing anything (camera didn't move).
                    if !frame_cache.write_some {
                        wrote_none = true;
                        break 'write;
                    }

                    buffer = &frame_cache.write_buffer;
                    camera_matrix = &renderer.camera.aligned.camera_matrix;
                    middle = renderer.camera.aligned.center;

                    Some(frame_cache.scrolled_frame.bind(renderer))
                } else {
                    None
                };

                shader.uniform_matrix3f("uCamera", camera_matrix);

                // Pack middle and derivative.
                // Derivative same between aligned/unaligned cameras.
                let derivative = renderer.camera.derivative();
                shader.uniform4f(
                    "uMiddle_uDerivative",
                    vec4(middle.x, middle.y, derivative, derivative),
                );

                // Set custom uniforms.
                self.context.prepare(renderer, &mut shader);

                buffer.bind(renderer).draw();
            } else {
                // First shader wasn't loaded so don't bother writing frame cache to main screen.
                return;
            }
        }

        // Read from frame cache and render to main screen.
        if let Some(frame_cache) = &mut self.frame_cache {
            if let Some(shader) = frame_cache.shader.bind(renderer) {
                // We can't render the scrolled frame directly to the main frame if there is a
                // subpixel that we have to adjust for because we can't interpolate the scrolled
                // frame by a subpixel. If the subpixel is zero we can render directly to the main
                // frame without going through the linear frame to provide linear interpolation.
                // This restriction could be worked around in WebGL2 since it allows repeating non
                // power of 2 textures (viewport). Ideally we would also pad the viewport width and
                // height by 1 so the interpolated border pixel isn't clamped (or repeating to
                // opposite side in WebGL2).
                let subpixel = renderer.camera.subpixel_uv_diff();
                let multipass = subpixel != Vec2::ZERO;

                // Don't need to copy if original wasn't modified (main frame is always modified).
                // Use &= to make sure to render to linear if skipped a frame due to not multipass.
                // If we wrote none and are multipass and linear is up to date we can skip drawing.
                frame_cache.linear_valid &= wrote_none && multipass;
                if !frame_cache.linear_valid {
                    shader.uniform_texture("uSampler", frame_cache.scrolled_frame.as_texture(), 0);

                    let fb_binding = multipass.then(|| {
                        // Drawing to whole linear so it will become valid.
                        frame_cache.linear_valid = true;
                        frame_cache.linear_frame.bind(renderer)
                    });

                    // Read buffer is used to read from frame cache.
                    frame_cache.read_buffer.bind(renderer).draw();

                    drop(fb_binding);
                }

                if multipass {
                    shader.uniform_texture("uSampler", frame_cache.linear_frame.as_texture(), 0);

                    let buffer = &mut frame_cache.subpixel_buffer;
                    buffer_viewport_with_uv(renderer, buffer, |uv| uv - subpixel);
                    buffer.bind(renderer).draw();
                }
            }
        }
    }
}

/// Cache for frames drawn by [`BackgroundLayer`].
///
/// Based on <https://www.factorio.com/blog/post/fff-333>.
struct FrameCache {
    shader: Shader,
    linear_frame: Framebuffer,
    linear_valid: bool, // Linear contains an up to date frame.
    scrolled_frame: Framebuffer,
    scroll: UVec2,
    read_buffer: TriangleBuffer<PosUv>,
    subpixel_buffer: TriangleBuffer<PosUv>,
    write_buffer: TriangleBuffer<PosUv>,
    write_some: bool,
}

impl FrameCache {
    fn new(renderer: &Renderer2d) -> Self {
        // Compile framebuffer shader (alternative to WebGL2's blitFramebuffer).
        let shader = renderer.create_shader(
            include_str!("shaders/framebuffer.vert"),
            include_str!("shaders/framebuffer.frag"),
        );

        let linear_frame = Framebuffer::new(renderer, true);
        let linear_valid = false;
        let scrolled_frame = Framebuffer::new(renderer, false);
        let scroll = UVec2::ZERO;

        let read_buffer = TriangleBuffer::new(renderer);
        let subpixel_buffer = TriangleBuffer::new(renderer);
        let write_buffer = TriangleBuffer::new(renderer);
        let write_some = false;

        Self {
            shader,
            linear_frame,
            linear_valid,
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
        renderer: &Renderer2d,
        maybe_delta: Option<IVec2>,
        invalidation: Option<Invalidation>,
    ) {
        // TODO don't set viewports during render phase beacause texture allocation stalls pipeline.
        let viewport = renderer.canvas_size();
        self.scrolled_frame.set_viewport(renderer, viewport);
        self.linear_frame.set_viewport(renderer, viewport);

        const DEBUG: bool = false;

        // Validate that a delta can be made.
        let delta = {
            if let Some(delta) = maybe_delta {
                if delta.abs().cmpge(viewport.as_ivec2()).any() {
                    if DEBUG {
                        console_log!("invalidated: out of range");
                    }
                    None
                } else if let Some(inv) = invalidation {
                    if DEBUG {
                        console_log!("invalidated: {:?}", inv);
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
                    console_log!("invalidated: zoom");
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
            renderer,
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
                            let matrix = &renderer.camera.aligned.view_matrix;

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
            console_log!("drawing {} pixels", pixels_drawn);
        }

        // Undo scroll when reading from buffer.
        let inv_scroll = viewport - scroll;
        Self::buffer_scrolled_rects(
            renderer,
            &mut self.read_buffer,
            Self::iter_scrolled_rects(UVec2::ZERO, viewport, inv_scroll, viewport),
            viewport,
        );
    }

    /// Triangulates and buffers an iterator of rects to a render buffer.
    /// The generated triangles aren't necessarily in the same order as the iterator.
    /// Returns if any triangles were generated.
    fn buffer_scrolled_rects(
        renderer: &Renderer2d,
        buffer: &mut TriangleBuffer<PosUv>,
        rects: impl Iterator<Item = (Rect, Rect)>,
        viewport: UVec2,
    ) -> bool {
        let viewport_rect = Rect {
            start: UVec2::ZERO,
            end: viewport,
        };
        let multiplier = viewport.as_vec2().recip() * 2.0;

        let mut mesh = MeshBuilder::new();
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
            buffer_viewport(renderer, buffer);
            true
        } else {
            if mesh.vertices.is_empty() {
                false
            } else {
                mesh.push_default_quads();
                buffer.buffer_mesh(renderer, &mesh);
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
        IntoIterator::into_iter([
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
