// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::attribs::Attribs;
use crate::gl::*;
use crate::renderer::Renderer;
use crate::vertex::Vertex;
use std::borrow::{Borrow, Cow};
use std::collections::VecDeque;
use std::mem::size_of;
use web_sys::{WebGlBuffer, WebGlVertexArrayObject};

/// It's analogous to [`VecDeque`] but on the GPU.
pub struct PointDeque<V> {
    // WebGL resources.
    vertices: WebGlBuffer,
    vao: WebGlVertexArrayObject,

    // Capacity, always a power of 2.
    capacity: usize,

    // Where data is read from vertices.
    tail: usize,

    // Where data is written to vertices.
    head: usize,

    // CPU buffer (required in WebGL because no copyBufferSubData).
    buffer: VecDeque<V>,

    // How many items were popped from the buffer since it was copied to the GPU.
    popped: usize,

    // How many items were pushed to the buffer since it was copied to the GPU.
    pushed: usize,
}

// Should be 1 less than a power of 2 because VecDeque skips 1 elem.
const STARTING_CAP: usize = 1023;

impl<V: Vertex> PointDeque<V> {
    /// Creates a new [`PointDeque`].
    pub fn new(renderer: &Renderer) -> Self {
        let gl = &renderer.gl;
        let ovao = &renderer.ovao;
        let deque = Self {
            vertices: gl.create_buffer().unwrap(),
            vao: ovao.create_vertex_array_oes().unwrap(),
            capacity: 0,
            tail: 0,
            head: 0,
            buffer: VecDeque::with_capacity(STARTING_CAP),
            popped: 0,
            pushed: 0,
        };

        // Make sure array was unbound.
        debug_assert!(gl
            .get_parameter(Ovao::VERTEX_ARRAY_BINDING_OES)
            .unwrap()
            .is_null());

        // Make sure binding was cleared.
        debug_assert!(gl
            .get_parameter(Gl::ARRAY_BUFFER_BINDING)
            .unwrap()
            .is_null());

        ovao.bind_vertex_array_oes(Some(&deque.vao));

        // Bind buffer to vao.
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&deque.vertices));
        V::bind_attribs(&mut Attribs::new::<V>(gl));

        // Unbind ALWAYS required (unlike all other render unbinds).
        ovao.bind_vertex_array_oes(None);

        // Unbind (not required in release mode).
        #[cfg(debug_assertions)]
        gl.bind_buffer(Gl::ARRAY_BUFFER, None);

        deque
    }

    /// Pushs a point to the [`PointDeque`] (silently errors once there are 1,000,000 points).
    pub fn push_back(&mut self, v: V) {
        if self.buffer.len() >= 1000000 {
            return;
        }
        self.pushed += 1;
        self.buffer.push_back(v);
    }

    /// Pops a point from the front of the [`PointDeque`], returning it.
    pub fn pop_front(&mut self) -> V {
        self.popped += 1;
        self.buffer.pop_front().unwrap()
    }

    /// Peeks the front of the [`PointDeque`].
    pub fn front(&self) -> Option<&V> {
        self.buffer.front()
    }

    /// Returns true if the [`PointDeque`] has no points to draw.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Binds the [`PointDeque`] to draw points.
    pub fn bind<'a>(&'a mut self, renderer: &'a Renderer) -> PointDequeBinding<'a, V> {
        self.buffer(renderer);
        PointDequeBinding::new(&renderer.gl, &renderer.ovao, self)
    }

    /// Called by bind.
    fn buffer(&mut self, renderer: &Renderer) {
        if self.pushed == 0 && self.popped == 0 {
            return;
        }
        let gl = &renderer.gl;

        // This can easily mess up the bind_buffer calls.
        debug_assert!(gl
            .get_parameter(Ovao::VERTEX_ARRAY_BINDING_OES)
            .unwrap()
            .is_null());

        // Make sure binding was cleared.
        debug_assert!(gl
            .get_parameter(Gl::ARRAY_BUFFER_BINDING)
            .unwrap()
            .is_null());

        // Buffer vertices.
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.vertices));

        // Allocate buffer to nearest power of 2 according to VecDeque.
        // self.buffer.capacity() + 1 because VecDeque skips 1 elem.
        // Never shrink buffer.
        let new_cap = self.buffer.capacity() + 1;
        assert_eq!(new_cap, new_cap.next_power_of_two());
        if new_cap > self.capacity {
            // Allocate new_cap on GPU.
            let bytes = (new_cap * size_of::<V>()) as i32;
            gl.buffer_data_with_i32(Gl::ARRAY_BUFFER, bytes, Gl::DYNAMIC_DRAW);
            self.capacity = new_cap;

            // All data was deleted by grow so reset head and tail.
            self.tail = 0;
            self.head = self.buffer.len();

            let (a, b) = self.buffer.as_slices();

            let mut offset = 0;
            for vertices in [a, b] {
                if vertices.is_empty() {
                    continue;
                }

                unsafe {
                    // Points to raw rust memory so can't allocate while in use.
                    let vert_array = js_sys::Float32Array::view(bytemuck::cast_slice(vertices));
                    gl.buffer_sub_data_with_i32_and_array_buffer_view(
                        Gl::ARRAY_BUFFER,
                        offset,
                        &vert_array,
                    );
                }

                offset += (vertices.len() * size_of::<V>()) as i32;
            }
        } else if self.popped > 0 || self.pushed > 0 {
            debug_assert_eq!(self.capacity, self.capacity.next_power_of_two());

            let n = self.buffer.len();

            let new_pushed = self.pushed.min(n);
            if new_pushed != self.pushed {
                // However many we skip out of pushed, we skip out of popped.
                self.popped = self.popped.saturating_sub(self.pushed - new_pushed)
            }

            // Only push items that are still in the buffer.
            self.pushed = new_pushed;

            // Capacity is power of 2 so & works as a faster %.
            self.tail = (self.tail + self.popped) & (self.capacity - 1);

            let range = n - self.pushed..n;
            let (slice_a, slice_b) = self.buffer.as_slices();

            let vertices = if slice_b.len() >= self.pushed {
                // Slice b aka last items has all pushed items.
                Cow::Borrowed(&slice_b[slice_b.len() - self.pushed..])
            } else if slice_b.is_empty() {
                // Slice b is empty aka contiguous so slice a has all items.
                Cow::Borrowed(&slice_a[range])
            } else {
                // Items are split across 2 slices so allocation is needed.
                Cow::Owned(self.buffer.range(range).copied().collect())
            };

            let vertices: &[V] = vertices.borrow();
            if !vertices.is_empty() {
                // Space after head available.
                let available = self.capacity - self.head;
                let split = vertices.len().min(available);

                let (slice_a, slice_b) = vertices.split_at(split);
                let calls = [(slice_a, self.head), (slice_b, 0)];

                for (slice, start) in calls {
                    if slice.is_empty() {
                        continue;
                    }

                    // Convert to bytes.
                    let offset = (start * size_of::<V>()) as i32;

                    unsafe {
                        // Points to raw rust memory so can't allocate while in use.
                        let vert_array = js_sys::Float32Array::view(bytemuck::cast_slice(slice));
                        gl.buffer_sub_data_with_i32_and_array_buffer_view(
                            Gl::ARRAY_BUFFER,
                            offset,
                            &vert_array,
                        );
                    }
                }
            }

            // Capacity is power of 2 so & works as a faster %.
            self.head = (self.head + self.pushed) & (self.capacity - 1);
        }

        self.pushed = 0;
        self.popped = 0;

        // Unbind (not required in release mode).
        #[cfg(debug_assertions)]
        gl.bind_buffer(Gl::ARRAY_BUFFER, None);
    }
}

/// A bound [`PointDeque`] that can draw points.
pub struct PointDequeBinding<'a, V: Vertex> {
    gl: &'a Gl,
    ovao: &'a Ovao,
    deque: &'a PointDeque<V>,
}

impl<'a, V: Vertex> PointDequeBinding<'a, V> {
    fn new(gl: &'a Gl, ovao: &'a Ovao, deque: &'a PointDeque<V>) -> Self {
        // Make sure buffer was unbound.
        debug_assert!(gl
            .get_parameter(Ovao::VERTEX_ARRAY_BINDING_OES)
            .unwrap()
            .is_null());

        ovao.bind_vertex_array_oes(Some(&deque.vao));
        Self { gl, ovao, deque }
    }

    /// Draws points.
    pub fn draw(&self) {
        if self.deque.tail <= self.deque.head {
            // Deque is contiguous.
            let points = self.deque.head - self.deque.tail;
            if points > 0 {
                self.gl
                    .draw_arrays(Gl::POINTS, self.deque.tail as i32, points as i32)
            }
        } else {
            // [tail, len)
            let points = self.deque.capacity - self.deque.tail;
            if points > 0 {
                self.gl
                    .draw_arrays(Gl::POINTS, self.deque.tail as i32, points as i32);
            }

            // [0, head)
            let points = self.deque.head;
            if points > 0 {
                self.gl.draw_arrays(Gl::POINTS, 0, points as i32);
            }
        }
    }
}

impl<'a, V: Vertex> Drop for PointDequeBinding<'a, V> {
    fn drop(&mut self) {
        // Unbind ALWAYS required (unlike all other render unbinds).
        self.ovao.bind_vertex_array_oes(None);
    }
}
