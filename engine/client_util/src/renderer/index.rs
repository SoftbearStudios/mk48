use bytemuck::Pod;
use js_sys::Object;
use std::fmt::Debug;
use std::ops::Deref;
use web_sys::WebGlRenderingContext as Gl;

pub trait Index: Pod + Debug + Default {
    type View: Deref<Target = Object>;

    fn from_usize(n: usize) -> Self;
    fn add(self, n: usize) -> Self;
    fn gl_enum() -> u32;
}

impl Index for u16 {
    type View = js_sys::Uint16Array;

    #[inline]
    fn from_usize(n: usize) -> Self {
        n as Self
    }

    #[inline]
    fn add(self, n: usize) -> Self {
        self + n as Self
    }

    #[inline]
    fn gl_enum() -> u32 {
        Gl::UNSIGNED_SHORT
    }
}

/// Must call renderer.enable_oes_element_index_uint().
impl Index for u32 {
    type View = js_sys::Uint32Array;

    #[inline]
    fn from_usize(n: usize) -> Self {
        n as Self
    }

    #[inline]
    fn add(self, n: usize) -> Self {
        self + n as Self
    }

    #[inline]
    fn gl_enum() -> u32 {
        Gl::UNSIGNED_INT
    }
}
