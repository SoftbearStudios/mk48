use crate::model::Model;
use crate::{Camera3d, Orthographic};
use glam::{IVec3, Vec2, Vec3};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

/// A triangle with a precomuted `normal`.
#[derive(Debug)]
struct Triangle {
    points: [Vec3; 3],
    normal: Vec3,
}

impl Triangle {
    fn depth(&self) -> f32 {
        Vec3::from(self.points.map(|p| p.z)).min_element()
    }

    fn iter_edges(&self) -> impl Iterator<Item = Edge> + '_ {
        let p = &self.points;
        [(p[0], p[1]), (p[1], p[2]), (p[2], p[0])]
            .into_iter()
            .map(|(start, end)| Edge::new(start, end))
    }

    fn visible(&self, camera: &Camera3d) -> bool {
        self.normal.dot(camera.normal()) < 0.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
struct Edge {
    start: HashVec3,
    end: HashVec3,
}

impl Edge {
    /// Creates a cannonical [`Edge`] (start < end).
    fn new(start: Vec3, end: Vec3) -> Self {
        let mut start = HashVec3(start);
        let mut end = HashVec3(end);
        if start > end {
            std::mem::swap(&mut start, &mut end);
        }
        Self { start, end }
    }
}

#[derive(Copy, Clone)]
struct HashVec3(Vec3);

impl HashVec3 {
    fn quantized(&self) -> IVec3 {
        // Lowers precision by 14 bits (required for normals).
        let precision = 1.0 / (1 << 14) as f32;
        self.0
            .as_ref()
            .map(|c| (c * (i32::MAX as f32 * precision)).round() as i32)
            .into()
    }
}

impl Deref for HashVec3 {
    type Target = Vec3;

    fn deref(&self) -> &Vec3 {
        &self.0
    }
}

// no NAN (because quantized)
impl Eq for HashVec3 {}

impl PartialEq for HashVec3 {
    fn eq(&self, other: &Self) -> bool {
        self.quantized().eq(&other.quantized())
    }
}

impl Hash for HashVec3 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.quantized().hash(state);
    }
}

// Doesn't implement Ord because compares f32s unlike Eq implementation which compares i32s.
impl PartialOrd for HashVec3 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.to_array().partial_cmp(&other.to_array()).unwrap())
    }
}

struct EdgeData {
    normals: HashSet<HashVec3>,
    visible_count: usize,
}

impl Model {
    fn triangles(&self) -> impl Iterator<Item = Triangle> + '_ {
        let vertex_floats = self.vertex_floats();
        self.indices
            .as_chunks()
            .0
            .into_iter()
            .map(move |indices: &[u16; 3]| {
                let points = indices
                    .map(|idx| Vec3::from_slice(&self.vertices[idx as usize * vertex_floats..]));
                let normal = (points[1] - points[0])
                    .cross(points[2] - points[0])
                    .normalize_or_zero();

                Triangle { points, normal }
            })
    }

    /// Creates an SVG data URL.
    /// TODO: take camera info as parameter.
    pub fn svg(&self) -> String {
        let camera = Camera3d::looking_at(
            Vec3::ONE.normalize(),
            Vec3::ZERO,
            Orthographic {
                dimensions: Vec3::splat(2.0),
            },
        );
        debug_assert!(camera
            .normal()
            .abs_diff_eq(-Vec3::ONE.normalize(), f32::EPSILON));

        let mut triangles: Vec<_> = self
            .triangles()
            .map(|t| {
                let points = t.points.map(|p| camera.world_to_ndc(p));
                debug_assert!(
                    points.iter().all(
                        |p| p.cmpge(Vec3::splat(-1.0)).all() && p.cmple(Vec3::splat(1.0)).all()
                    ),
                    "triangle outside camera {:?} -> {:?}",
                    t.points,
                    points
                );
                Triangle {
                    points,
                    normal: t.normal,
                }
            })
            .collect();

        // Depth sorting.
        triangles.sort_by_key(|t| (t.depth() * (i32::MIN as f32)) as i32);

        let mut edges = HashMap::<Edge, EdgeData>::new();

        for triangle in &triangles {
            let visible = triangle.visible(&camera);

            for edge in triangle.iter_edges() {
                let edge_data = edges.entry(edge).or_insert_with(|| EdgeData {
                    normals: Default::default(),
                    visible_count: 0,
                });

                edge_data.normals.insert(HashVec3(triangle.normal));
                edge_data.visible_count += visible as usize;
            }
        }

        // Render svg.
        let mut svg = String::new();
        svg.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-1 -1 2 2" fill="red" stroke="black" stroke-width="0.05" stroke-linecap="round">"#);

        for triangle in &triangles {
            // Cull backfacing triangles.
            if !triangle.visible(&camera) {
                continue;
            }

            // Color triangle with diffuse and ambient lighting.
            const LIGHT: Vec3 = Vec3::Y;
            let light = triangle.normal.dot(LIGHT).max(0.0) * 0.5 + 0.5;
            write!(
                &mut svg,
                r#"<path fill="{0}" stroke="{0}" stroke-width="0.015" d="M "#,
                renderer::rgba_array_to_css([(light * u8::MAX as f32) as u8, 0, 0, 255])
            )
            .unwrap();

            // Add points of triangle.
            let points = triangle.points;
            for point in points {
                let point = to_svg_space(point);
                write!(&mut svg, "{},{} ", point.x, point.y).unwrap();
            }

            // End triangle.
            svg.push_str(r#"z"/>"#);

            for edge in triangle.iter_edges() {
                if let Some(data) = edges.get_mut(&edge) {
                    if data.normals.len() <= 1 {
                        continue;
                    }
                    data.visible_count -= 1;
                    if data.visible_count != 0 {
                        continue;
                    }

                    let Edge { start, end } = edge;
                    let start = to_svg_space(*start);
                    let end = to_svg_space(*end);

                    write!(
                        svg,
                        r#"<path fill="none" stroke="black" d="M {},{} {},{}"/>"#,
                        start.x, start.y, end.x, end.y,
                    )
                    .unwrap();
                }
            }
        }

        svg.push_str("</svg>");
        let encoded = format!("data:image/svg+xml;base64,{}", base64::encode(svg));
        encoded
    }
}

fn to_svg_space(pos: Vec3) -> Vec2 {
    // viewbox
    let mut pos = pos.truncate();
    // SVG vertical coordinate reversed.
    pos.y = -pos.y;
    pos
}
