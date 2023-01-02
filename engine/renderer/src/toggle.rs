use crate::{Layer, RenderLayer, Renderer};

/// A [`Layer`] that can be enabled and disabled with a [`prim@bool`].
pub struct ToggleLayer<I> {
    /// The inner [`Layer`] passed to [`new`][`Self::new`].
    pub inner: I,
    /// If the [`ToggleLayer`] should render (`false` by default).
    pub enabled: bool,
}

impl<I> ToggleLayer<I> {
    /// Creates a new [`ToggleLayer`] with an `inner` [`Layer`].
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            enabled: false,
        }
    }
}

impl<I: Layer> Layer for ToggleLayer<I> {
    fn pre_prepare(&mut self, renderer: &Renderer) {
        self.inner.pre_prepare(renderer);
    }

    fn pre_render(&mut self, renderer: &Renderer) {
        self.inner.pre_render(renderer);
    }
}

impl<I: RenderLayer<P>, P> RenderLayer<P> for ToggleLayer<I> {
    fn render(&mut self, renderer: &Renderer, params: P) {
        if self.enabled {
            self.inner.render(renderer, params);
        }
    }
}
