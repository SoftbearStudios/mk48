// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::camera_2d::Camera2d;
use glam::{uvec2, vec2, IVec2, UVec2, Vec2};
use js_hooks::console_log;
use renderer::{
    derive_vertex, DefaultRender, Framebuffer, Layer, MeshBuilder, RenderLayer, Renderer, Shader,
    ShaderBinding, TriangleBuffer,
};
use std::ops::Range;

/// Invalidates part of the [`BackgroundLayer`]'s cached frame. Useful if uniforms change.
#[derive(Debug)]
pub enum Invalidation {
    /// Redraw the whole buffer.
    All,
    /// Redraw only the part of the buffer covered by some rectangles.
    /// Coordinates are in world space.
    Rects(Vec<(Vec2, Vec2)>),
}

derive_vertex!(
    struct PosUv {
        pos: Vec2,
        uv: Vec2,
    }
);

/// Draws a [`Shader`] over the whole screen. Must not override `uniform mat3 uCamera`. The
/// [`Shader`] must take `uniform mat3 uCamera;`.
///
/// If the an [`Option<Invalidation>`] is passed [`BackgroundLayer`] will cache the frame with a
/// [`Framebuffer`]. Only use this on expensive [`Shader`]s that don't make [`Invalidation`]s that
/// often for performance gains. The [`Option<Invalidation>`] reports which pixels were invalidated
/// by changes to uniforms.
///
/// Its implementation of [`RenderLayer`] takes a [`ShaderBinding`] by value since it has to unbind
/// it to use its internal [`Shader`]s.
pub struct BackgroundLayer {
    buffer: TriangleBuffer<PosUv, u16>,
    frame_cache: Option<FrameCache>,
}

impl DefaultRender for BackgroundLayer {
    fn new(renderer: &Renderer) -> Self {
        let mut buffer = TriangleBuffer::new(renderer);
        buffer_viewport(renderer, &mut buffer);
        Self {
            buffer,
            frame_cache: None,
        }
    }
}

/// Buffers a single triangle that covers the whole viewport.
fn buffer_viewport(renderer: &Renderer, buffer: &mut TriangleBuffer<PosUv>) {
    buffer_viewport_with_uv(renderer, buffer, |uv| uv);
}

/// Buffers a single triangle that covers the whole viewport.
/// Calls the provided function to get the uv from the screen space position.
fn buffer_viewport_with_uv<F: FnMut(Vec2) -> Vec2>(
    renderer: &Renderer,
    buffer: &mut TriangleBuffer<PosUv>,
    mut f: F,
) {
    buffer.buffer(
        renderer,
        &[vec2(-1.0, 3.0), vec2(-1.0, -1.0), vec2(3.0, -1.0)].map(|pos| PosUv { pos, uv: f(pos) }),
        &[],
    );
}

// TODO set fb viewport in pre_render.
impl Layer for BackgroundLayer {}

impl RenderLayer<(ShaderBinding<'_>, &Camera2d, Option<Option<Invalidation>>)> for BackgroundLayer {
    fn render(
        &mut self,
        renderer: &Renderer,
        (shader, camera, invalidation): (ShaderBinding, &Camera2d, Option<Option<Invalidation>>),
    ) {
        let mut wrote_none: bool = false;

        // Write to frame cache or main screen.
        'write: {
            let mut buffer = &self.buffer;
            let mut camera_matrix = &camera.camera_matrix;

            let _fbb = if let Some(invalidation) = invalidation {
                let frame_cache = self
                    .frame_cache
                    .get_or_insert_with(|| FrameCache::new(renderer));

                // Update the frame cache (resize texture and compute read/write buffers).
                frame_cache.update(
                    renderer,
                    camera,
                    camera.aligned.delta_pixels,
                    invalidation.as_ref(),
                );

                // Not drawing anything (camera didn't move).
                if !frame_cache.write_some {
                    wrote_none = true;
                    break 'write;
                }

                buffer = &frame_cache.write_buffer;
                camera_matrix = &camera.aligned.camera_matrix;

                Some(frame_cache.scrolled_frame.bind(renderer))
            } else {
                assert!(self.frame_cache.is_none(), "stopped caching frame");
                None
            };

            shader.uniform("uCamera", camera_matrix);
            buffer.bind(renderer).draw();
        }
        drop(shader); // Unbind shader so we can bind another one.

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
                let subpixel = camera.subpixel_uv_diff();
                let multipass = subpixel != Vec2::ZERO;

                // Don't need to copy if original wasn't modified (main frame is always modified).
                // Use &= to make sure to render to linear if skipped a frame due to not multipass.
                // If we wrote none and are multipass and linear is up to date we can skip drawing.
                frame_cache.linear_valid &= wrote_none && multipass;
                if !frame_cache.linear_valid {
                    shader.uniform("uSampler", frame_cache.scrolled_frame.as_texture());

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
                    shader.uniform("uSampler", frame_cache.linear_frame.as_texture());

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
    empty: bool,
    linear_frame: Framebuffer,
    linear_valid: bool, // Linear contains an up to date frame.
    read_buffer: TriangleBuffer<PosUv>,
    scroll: UVec2,
    scrolled_frame: Framebuffer,
    shader: Shader,
    subpixel_buffer: TriangleBuffer<PosUv>,
    write_buffer: TriangleBuffer<PosUv>,
    write_some: bool,
}

impl DefaultRender for FrameCache {
    fn new(renderer: &Renderer) -> Self {
        // Compile framebuffer shader (alternative to WebGL2's blitFramebuffer).
        let shader = renderer.create_shader(
            include_str!("shaders/framebuffer.vert"),
            include_str!("shaders/framebuffer.frag"),
        );

        // Use zero background colors because the real shader will be compiled before copying to the
        // screen.

        let linear_frame = Framebuffer::new(renderer, [0; 4], true);
        let linear_valid = false;
        let scrolled_frame = Framebuffer::new(renderer, [0; 4], false);
        let scroll = UVec2::ZERO;

        let read_buffer = TriangleBuffer::new(renderer);
        let subpixel_buffer = TriangleBuffer::new(renderer);
        let write_buffer = TriangleBuffer::new(renderer);
        let write_some = false;

        Self {
            empty: true,
            linear_frame,
            linear_valid,
            read_buffer,
            scroll,
            scrolled_frame,
            shader,
            subpixel_buffer,
            write_buffer,
            write_some,
        }
    }
}

impl FrameCache {
    fn update(
        &mut self,
        renderer: &Renderer,
        camera: &Camera2d,
        maybe_delta: Option<IVec2>,
        invalidation: Option<&Invalidation>,
    ) {
        // TODO don't set viewports during render phase because texture allocation stalls pipeline.
        let viewport = renderer.canvas_size();
        self.scrolled_frame.set_viewport(renderer, viewport);
        self.linear_frame.set_viewport(renderer, viewport);

        const DEBUG: bool = false;

        // Validate that a delta can be made.
        let delta = {
            if std::mem::take(&mut self.empty) {
                if DEBUG {
                    console_log!("invalidated: empty");
                }
                None
            } else if let Some(delta) = maybe_delta {
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
                        Invalidation::Rects(rects) => Some((delta, rects.as_slice())),
                    }
                } else {
                    Some((delta, [].as_slice()))
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
                        .chain(delta_rects.into_iter().flat_map(|&rect| {
                            let (start, end) = rect;
                            let matrix = &camera.aligned.view_matrix;

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
        renderer: &Renderer,
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
                .filter_map(|(rect, scrolled)| {
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
                                (end, end2),
                                (uvec2(start.x, end.y), uvec2(start2.x, end2.y)),
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
                .flatten(),
        );

        if single_triangle {
            buffer_viewport(renderer, buffer);
            true
        } else if mesh.vertices.is_empty() {
            false
        } else {
            mesh.push_default_quads();
            buffer.buffer_mesh(renderer, &mesh);
            true
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
        let x_range = rect_start.x.saturating_sub(inv_scroll.x)
            ..scroll.x.saturating_sub(viewport.x - rect_end.x);
        let x_range2 = (scroll.x + rect_start.x).min(viewport.x)
            ..(viewport.x - inv_scroll.x.saturating_sub(rect_end.x));
        let y_range = rect_start.y.saturating_sub(inv_scroll.y)
            ..scroll.y.saturating_sub(viewport.y - rect_end.y);
        let y_range2 = (scroll.y + rect_start.y).min(viewport.y)
            ..(viewport.y - inv_scroll.y.saturating_sub(rect_end.y));

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
    assert!(b.unsigned_abs() <= snap);

    if b >= 0 {
        let r = a + b as u32;
        if r > snap {
            r - snap
        } else {
            r
        }
    } else {
        snapping_sub(a, b.unsigned_abs(), snap, false)
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
