use glam::{dvec2, DMat3, DVec2, IVec2, Mat3, UVec2, Vec2};

#[derive(Default, Clone)]
pub struct Camera {
    center: DVec2,
    zoom: f64,
    aligned: bool,                   // Aligned to pixels.
    pub delta_pixels: Option<IVec2>, // None = invalidated, Some = moved excact pixels.
    pub viewport: UVec2,
    pub camera_matrix: Mat3,
    pub view_matrix: Mat3,
}

impl Camera {
    /// Creates a new camera.
    /// If the camera is aligned it snaps to the nearest pixel.
    pub fn new(aligned: bool) -> Self {
        Self {
            aligned,
            ..Default::default()
        }
    }

    pub fn aspect_ratio(&self) -> f32 {
        let [width, height] = self.viewport.as_vec2().to_array();
        width / height
    }

    fn aspect_ratio_f64(&self) -> f64 {
        let [width, height] = self.viewport.as_dvec2().to_array();
        width / height
    }

    pub fn center(&self) -> Vec2 {
        self.center.as_vec2()
    }

    /// Returns the subpixel difference between two cameras as a viewport uv.
    /// Useful for reversing camera alignment.
    pub(crate) fn subpixel_uv_diff(&self, other: &Self) -> Vec2 {
        assert_eq!(self.viewport, other.viewport);
        let diff = self.center - other.center;
        self.view_matrix.transform_vector2(diff.as_vec2())
    }

    pub fn zoom(&self) -> f32 {
        self.zoom as f32
    }

    pub fn update(&mut self, center: Vec2, zoom: f32, viewport: UVec2) {
        let center = center.as_dvec2();
        let zoom = zoom as f64;

        if self.aligned {
            let zoom_changed = self.zoom != zoom;
            if zoom_changed {
                self.zoom = zoom;
            }

            let viewport_changed = self.viewport != viewport;
            if viewport_changed {
                self.viewport = viewport;
            }

            let (aligned, delta) = Self::round_to_pixel(center, self.center, zoom, viewport);

            self.center = aligned;
            self.delta_pixels = (!zoom_changed && !viewport_changed).then_some(delta);
        } else {
            self.zoom = zoom;
            self.center = center;
            self.viewport = viewport
        }

        // This matrix is the camera matrix manually inverted.
        let view_matrix = DMat3::from_scale(dvec2(1.0, self.aspect_ratio_f64()) / zoom)
            .mul_mat3(&DMat3::from_translation(-self.center));

        self.view_matrix = view_matrix.as_mat3();
        self.camera_matrix = view_matrix.inverse().as_mat3();
    }

    fn round_to_pixel(mut pos: DVec2, prev: DVec2, zoom: f64, viewport: UVec2) -> (DVec2, IVec2) {
        let viewport = viewport.as_dvec2();
        // Viewport is -1 to 1, so must double.
        let scale = dvec2(zoom, zoom * viewport.y / viewport.x) * 2.0;

        // Meters per pixel and pixels per meter.
        let mpp = scale / viewport;
        let ppm = viewport / scale;

        // Round center to nearest pixel.
        pos = (pos * ppm).round() * mpp;

        let deltaf = (pos - prev) * ppm;
        let delta = deltaf.round().as_ivec2();

        (pos, delta)
    }
}
