use glam::{uvec2, UVec2};
use std::fmt;
use std::fmt::Formatter;

/// A 2D [`Vec`] of [`prim@bool`].
#[derive(Default)]
pub struct Mask {
    /// Y-major order. Length must be `dim.x * dim.y`.
    mask: Vec<bool>,
    dim: UVec2,
}

impl Mask {
    /// Creates a [`Mask`] that expands each point to a square `kernel` around each point. Even
    /// `kernel`s will add more than they subtract. All `points` must be less than `dim`. `kernel`
    /// must be >= 0.
    pub fn new_expanded(points: impl IntoIterator<Item = UVec2>, dim: UVec2, kernel: u32) -> Self {
        assert_ne!(kernel, 0);

        let sub = (kernel - 1) / 2;
        let add = kernel / 2 + 1;

        let mut mask = vec![false; (dim.x * dim.y) as usize];
        for p in points {
            assert!(p.cmple(dim).all());

            for y in p.y.saturating_sub(sub)..(p.y + add).min(dim.y) {
                for x in p.x.saturating_sub(sub)..(p.x + add).min(dim.x) {
                    *index_2d_mut(&mut mask, dim, x, y) = true;
                }
            }
        }

        Self { mask, dim }
    }

    /// Converts a [`Mask`] into an iterator of rectangles that cover it. Mask is in y major order
    /// (same as texture). Rects are inclusive start/end points. This is useful for generating
    /// [`Invalidation`][`crate::background::Invalidation`]s.
    pub fn into_rects(self) -> impl Iterator<Item = (UVec2, UVec2)> {
        let Self { mut mask, dim } = self;

        // Preserve a copy of mask before it's modified in debug mode for assertions.
        #[cfg(debug_assertions)]
        let mask1 = mask.clone();

        // Use greedy meshing algorithm.
        let mut rects = vec![];
        for y in 0..dim.y {
            let mut x = 0;

            while x < dim.x {
                if index_2d(&mask, dim, x, y) {
                    let mut maybe_x2 = None;
                    for y2 in y..(dim.y + 1) {
                        let i = (y2 < dim.y)
                            .then(|| {
                                index_1d(&mask, dim, y2)
                                    [x as usize..=maybe_x2.unwrap_or(dim.x - 1) as usize]
                                    .iter()
                                    .enumerate()
                                    .take_while(|(_, &v)| v)
                                    .map(|(i, _)| i)
                                    .last()
                                    .map(|i| i as u32)
                            })
                            .flatten();

                        if let Some(x2) = maybe_x2 {
                            if i.map(|i| i + x) != Some(x2) {
                                rects.push((uvec2(x, y), uvec2(x2, y2 - 1)));
                                x = x2;
                                break;
                            }
                        } else if let Some(i) = i {
                            maybe_x2 = Some(i + x);
                        } else {
                            break;
                        }

                        index_1d_mut(&mut mask, dim, y2)[x as usize..=(x + i.unwrap()) as usize]
                            .fill(false);
                    }
                }

                x += 1;
            }
        }

        // Check results in debug mode.
        #[cfg(debug_assertions)]
        {
            let mut mask2 = vec![false; mask.len()];
            for (s, e) in rects.iter().copied() {
                for y in s.y..=e.y {
                    for x in s.x..=e.x {
                        *index_2d_mut(&mut mask2, dim, x, y) = true;
                    }
                }
            }

            if mask1 != mask2 {
                let mut s = String::from("mask1\n");
                s += &format!("{mask1:?}");
                s += "mask2\n";
                s += &&format!("{mask2:?}");
                let mut diff = mask1;
                for y in 0..dim.y {
                    for x in 0..dim.x {
                        *index_2d_mut(&mut diff, dim, x, y) =
                            index_2d(&diff, dim, x, y) != index_2d(&mask2, dim, x, y);
                    }
                }
                s += "diff\n";
                s += &format!("{diff:?}");
                panic!("{}", s);
            }
        }
        rects.into_iter()
    }
}

impl fmt::Debug for Mask {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        assert_eq!(self.mask.len(), (self.dim.y * self.dim.x) as usize);
        for y in 0..self.dim.y {
            for x in 0..self.dim.x {
                let v = index_2d(&self.mask, self.dim, x, y);
                write!(f, "{}", (b'0' + v as u8) as char)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

fn index_1d(m: &[bool], dim: UVec2, y: u32) -> &[bool] {
    &m[(y * dim.x) as usize..((y + 1) * dim.x) as usize]
}

fn index_2d(m: &[bool], dim: UVec2, x: u32, y: u32) -> bool {
    m[(y * dim.x + x) as usize]
}

fn index_1d_mut(m: &mut [bool], dim: UVec2, y: u32) -> &mut [bool] {
    &mut m[(y * dim.x) as usize..((y + 1) * dim.x) as usize]
}

fn index_2d_mut(m: &mut [bool], dim: UVec2, x: u32, y: u32) -> &mut bool {
    &mut m[(y * dim.x + x) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_mask1() {
        let points = [uvec2(0, 0), uvec2(1, 1)];
        let dim = UVec2::splat(2);
        let kernel = 1;

        let rects = [(uvec2(0, 0), uvec2(0, 0)), (uvec2(1, 1), uvec2(1, 1))];

        let mask = Mask::new_expanded(points, dim, kernel);
        assert_eq!(
            format!("{mask:?}"),
            "\
            10\n\
            01\n\
            "
        );
        let res: HashSet<_> = mask.into_rects().collect();
        assert_eq!(res, rects.into())
    }

    #[test]
    fn test_mask2() {
        let points = [uvec2(1, 0), uvec2(0, 1), uvec2(1, 1)];
        let dim = UVec2::splat(3);
        let kernel = 3;

        let rects = [(uvec2(0, 0), uvec2(2, 2))];

        let mask = Mask::new_expanded(points, dim, kernel);
        assert_eq!(
            format!("{mask:?}"),
            "\
            111\n\
            111\n\
            111\n\
            "
        );
        let res: HashSet<_> = mask.into_rects().collect();
        assert_eq!(res, rects.into())
    }

    #[test]
    fn test_mask3() {
        let points = [uvec2(1, 0), uvec2(0, 1), uvec2(1, 1)];
        let dim = UVec2::splat(3);
        let kernel = 2;

        let rects = [(uvec2(1, 0), uvec2(2, 2)), (uvec2(0, 1), uvec2(0, 2))];

        let mask = Mask::new_expanded(points, dim, kernel);
        assert_eq!(
            format!("{mask:?}"),
            "\
            011\n\
            111\n\
            111\n\
            "
        );
        let res: HashSet<_> = mask.into_rects().collect();
        assert_eq!(res, rects.into())
    }
}
