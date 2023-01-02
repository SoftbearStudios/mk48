use glam::{const_vec3, vec3, Mat4, Vec3};

/// An axis-aligned bounding box. Points that are between `min` and `max` are considered inside it.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Aabb3 {
    /// The smallest values that are considered inside the [`Aabb3`].
    pub min: Vec3,
    /// The largest values that are considered inside the [`Aabb3`].
    pub max: Vec3,
}

impl Aabb3 {
    /// An [`Aabb3`] that contains all points.
    pub const UNBOUNDED: Self = Self {
        min: const_vec3!([f32::NEG_INFINITY; 3]),
        max: const_vec3!([f32::INFINITY; 3]),
    };

    /// Creates a new [`Aabb3`] from `min` and `max` points.
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Creates a new [`Aabb3`] from a `center` point and `dimensions`.
    pub fn from_center_and_dimensions(center: Vec3, dimensions: Vec3) -> Self {
        Self {
            min: center - dimensions * 0.5,
            max: center + dimensions * 0.5,
        }
    }

    /// Returns if `point` is between `self.min` and `self.max`.
    pub fn contains(&self, point: Vec3) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    /// Transforms the [`Aabb3`]'s points and creates a new [`Aabb3`] from them. If `matrix` is only
    /// a rotation + transformation, the resulting [`Aabb3`]'s area must be >= than the original.
    pub fn transformed_by(&self, matrix: &Mat4) -> Self {
        self.vertices()
            .into_iter()
            .map(|v| matrix.transform_point3(v))
            .collect()
    }

    /// Returns the 8 corners of the [`Aabb3`]` in an arbitrary order.
    pub fn vertices(&self) -> [Vec3; 8] {
        let a = self.min;
        let b = self.max;

        [
            a,
            vec3(b.x, a.y, a.z),
            vec3(a.x, b.y, a.z),
            vec3(b.x, b.y, a.z),
            vec3(a.x, a.y, b.z),
            vec3(b.x, a.y, b.z),
            vec3(a.x, b.y, b.z),
            b,
        ]
    }
}

impl FromIterator<Vec3> for Aabb3 {
    fn from_iter<T: IntoIterator<Item = Vec3>>(iter: T) -> Self {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for v in iter {
            min = v.min(min);
            max = v.max(max);
        }

        Self { min, max }
    }
}
