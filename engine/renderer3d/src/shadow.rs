use crate::Camera3d;
use glam::{Mat4, UVec2, Vec3};
use renderer::{Framebuffer, Layer, RenderLayer, Renderer, ShaderBinding, Texture};
use std::ops::Deref;

/// The parameters required to render to a shadow map.
pub struct ShadowParams {
    /// The camera of the shadow map.
    pub camera: Camera3d,
}

/// The results of [`render_shadows`][`ShadowCaster::render_shadows`]. [`Deref`]s to `params`.
pub struct ShadowResult<P> {
    /// The inner parameters.
    pub params: P,
    /// The matrix to transform world space to shadow space.
    pub shadow_matrix: Mat4,
    /// The [`Texture`] that the shadows were rendered to. Is [`None`] if `viewport` was [`None`].
    pub shadow_texture: Option<Texture>,
}

impl<P> ShadowResult<P> {
    /// Sets `uniform mat4 uShadowMatrix;` and `uniform highp sampler2DShadow uShadow;` if
    /// `shadow_texture` isn't [`None`]. Returns `true` if uniforms were set.
    pub fn prepare_shadows(&self, shader: &ShaderBinding) -> bool {
        if let Some(shadow_texture) = &self.shadow_texture {
            shader.uniform("uShadowMatrix", &self.shadow_matrix);
            shader.uniform("uShadow", shadow_texture);
            true
        } else {
            false
        }
    }
}

// Deref is the only way...
impl<P> Deref for ShadowResult<P> {
    type Target = P;
    fn deref(&self) -> &Self::Target {
        &self.params
    }
}

/// Renders the `inner`'s shadows and then renders the `inner` with shadows.
#[derive(Layer)]
pub struct ShadowLayer<L> {
    /// The [`Camera3d`] that will be used to render shadows.
    pub camera: Camera3d,
    framebuffer: Option<Framebuffer>,
    /// The [`Layer`] passed to [`with_inner_and_camera`][`Self::with_inner_and_camera`].
    #[layer]
    pub inner: L,
}

impl<L> ShadowLayer<L> {
    /// Creates a new [`ShadowLayer`] with a given `viewport`. If the `viewport` is [`None`] no
    /// shadows will be rendered.
    pub fn with_viewport(renderer: &Renderer, inner: L, viewport: Option<UVec2>) -> Self {
        // TODO change shadow resoultion based on shadow camera.
        let framebuffer = viewport.map(|viewport| {
            let mut framebuffer = Framebuffer::new_depth(renderer);
            framebuffer.set_viewport(renderer, viewport);
            framebuffer
        });

        Self {
            camera: Default::default(),
            framebuffer,
            inner,
        }
    }
}

impl<L, P> RenderLayer<P> for ShadowLayer<L>
where
    L: for<'a> RenderLayer<&'a ShadowParams> + for<'a> RenderLayer<&'a ShadowResult<P>>,
{
    fn render(&mut self, renderer: &Renderer, params: P) {
        let shadow_texture = self.framebuffer.as_mut().map(|fb| {
            let shadow_params = ShadowParams {
                camera: self.camera.clone(),
            };

            // Render fresh shadows.
            let fbb = fb.bind(renderer);
            fbb.clear();
            renderer.set_color_mask(false);
            renderer.set_cull_face(true);

            self.inner.render(renderer, &shadow_params);

            renderer.set_cull_face(false);
            renderer.set_color_mask(true);
            drop(fbb);

            fb.as_depth_texture().clone()
        });

        // Convert from ndc to uv.
        let shadow_matrix = Mat4::from_translation(Vec3::splat(0.5))
            * Mat4::from_scale(Vec3::splat(0.5))
            * self.camera.vp_matrix;

        let params = ShadowResult {
            params,
            shadow_matrix,
            shadow_texture,
        };

        // Render scene.
        self.inner.render(renderer, &params);
    }
}
