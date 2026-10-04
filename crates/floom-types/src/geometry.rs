//! Geometry value types.

/// 2D point in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    #[inline]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Euclidean distance between the two points.
    #[inline]
    pub fn distance_to(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}

/// 2D size in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    #[inline]
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

/// Axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    pub const ZERO: Self = Self {
        x0: 0.0,
        y0: 0.0,
        x1: 0.0,
        y1: 0.0,
    };

    #[inline]
    pub const fn from_origin_size(origin: Point, size: Size) -> Self {
        Self {
            x0: origin.x,
            y0: origin.y,
            x1: origin.x + size.width,
            y1: origin.y + size.height,
        }
    }

    #[inline]
    pub fn width(self) -> f64 {
        self.x1 - self.x0
    }

    #[inline]
    pub fn height(self) -> f64 {
        self.y1 - self.y0
    }

    #[inline]
    pub fn size(self) -> Size {
        Size::new(self.width(), self.height())
    }

    #[inline]
    pub fn origin(self) -> Point {
        Point::new(self.x0, self.y0)
    }

    #[inline]
    /// Centre of the rect.
    ///
    /// Where a rect collapses to the one point that represents it: the sample
    /// a gradient is reduced to when it has to be drawn as a single colour,
    /// and the pivot a rotation about the rect's middle turns on.
    pub fn center(self) -> Point {
        Point::new((self.x0 + self.x1) / 2.0, (self.y0 + self.y1) / 2.0)
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            x0: self.x0.min(other.x0),
            y0: self.y0.min(other.y0),
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
        }
    }

    #[inline]
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x0 && p.x < self.x1 && p.y >= self.y0 && p.y < self.y1
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    /// The overlap of two rects, or [`Rect::ZERO`] when they miss each other.
    ///
    /// Empty rather than inverted: an "intersection" that runs past its inputs
    /// (`x1 < x0`) flows into a native view's geometry and comes out the other
    /// side as a negative extent, which is a crash on one platform and a
    /// zero-sized window on another.
    #[inline]
    pub fn intersect(self, other: Self) -> Self {
        let x0 = self.x0.max(other.x0);
        let y0 = self.y0.max(other.y0);
        let x1 = self.x1.min(other.x1);
        let y1 = self.y1.min(other.y1);
        if x1 <= x0 || y1 <= y0 {
            Self::ZERO
        } else {
            Self { x0, y0, x1, y1 }
        }
    }

    /// Whether `other` lies entirely inside this rect. Used to skip a clip that
    /// does not actually bite, so "nothing clips me" stays distinguishable from
    /// "clipped to nothing".
    #[inline]
    pub fn contains_rect(self, other: Self) -> bool {
        other.x0 >= self.x0 && other.y0 >= self.y0 && other.x1 <= self.x1 && other.y1 <= self.y1
    }
}

/// Edge insets (top, right, bottom, left).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Insets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Insets {
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    #[inline]
    pub const fn uniform(v: f64) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }
}

/// A length along one axis: a fixed number of logical pixels, or a fraction of
/// the containing block.
///
/// A percentage is not a convenience — it is a different thing to measure, and
/// the difference is the whole reason a full-width row can fill its parent while
/// a fixed 200px row cannot. v1 had `width(..)` and `width_pct(..)` as two
/// builders over one field, and 2.0 had only the first, so the 310
/// `width_pct` / `height_pct` calls across floem-ui had no spelling at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    /// A fixed extent in logical pixels.
    Px(f32),
    /// A percentage of the containing block, `0.0..=100.0`.
    Pct(f32),
}

impl Length {
    /// The same length as a fraction of the containing block.
    pub const fn pct(percent: f32) -> Self {
        Self::Pct(percent)
    }

    /// The fixed part, if this is one. `None` for a percentage, which has no
    /// pixel value until the containing block is known — so a caller that needs a
    /// number has to say what it will do with the answer rather than get a
    /// silently wrong one.
    pub const fn px(self) -> Option<f32> {
        match self {
            Self::Px(px) => Some(px),
            Self::Pct(_) => None,
        }
    }

    pub const fn is_pct(self) -> bool {
        matches!(self, Self::Pct(_))
    }
}

/// Layout constraints (min/max size).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraints {
    pub min: Size,
    pub max: Size,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            min: Size::ZERO,
            max: Size::new(f64::INFINITY, f64::INFINITY),
        }
    }
}

impl Constraints {
    #[inline]
    pub fn tight(size: Size) -> Self {
        Self {
            min: size,
            max: size,
        }
    }

    #[inline]
    pub fn loosen(self) -> Self {
        Self {
            min: Size::ZERO,
            max: self.max,
        }
    }

    #[inline]
    pub fn constrain(self, size: Size) -> Size {
        Size::new(
            size.width.clamp(self.min.width, self.max.width),
            size.height.clamp(self.min.height, self.max.height),
        )
    }
}

/// 2D affine transform: `[a c e; b d f; 0 0 1]` column-major style matching common UI matrices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    #[inline]
    pub const fn translate(tx: f64, ty: f64) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    /// Uniform-free axis scaling about the origin.
    #[inline]
    pub const fn scale(sx: f64, sy: f64) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Scaling about a fixed point: `translate(origin) ∘ scale ∘ translate(-origin)`.
    /// This is what "scale this subtree around its own centre" means.
    #[inline]
    pub fn scale_around(sx: f64, sy: f64, origin: Point) -> Self {
        Self::translate(origin.x, origin.y)
            .multiply(Self::scale(sx, sy))
            .multiply(Self::translate(-origin.x, -origin.y))
    }

    /// Rotation by `radians` (screen coordinates: positive turns clockwise,
    /// because y points down).
    #[inline]
    pub fn rotate(radians: f64) -> Self {
        let (s, c) = radians.sin_cos();
        Self {
            a: c,
            b: s,
            c: -s,
            d: c,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Rotation about a fixed point — the pivot a spinner or a card flip
    /// actually spins around.
    #[inline]
    pub fn rotate_around(radians: f64, origin: Point) -> Self {
        Self::translate(origin.x, origin.y)
            .multiply(Self::rotate(radians))
            .multiply(Self::translate(-origin.x, -origin.y))
    }

    /// Matrix product `self ∘ other`: apply `other` to the point *first*,
    /// then `self`. `(a.multiply(b)).map_point(p) == a.map_point(b.map_point(p))`.
    #[inline]
    pub fn multiply(self, other: Self) -> Self {
        Self {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            e: self.a * other.e + self.c * other.f + self.e,
            f: self.b * other.e + self.d * other.f + self.f,
        }
    }

    /// The transform that undoes this one; `None` when it is singular
    /// (a zero scale has no inverse — nothing maps back).
    pub fn inverse(self) -> Option<Self> {
        let det = self.a * self.d - self.b * self.c;
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        let inv = 1.0 / det;
        Some(Self {
            a: self.d * inv,
            b: -self.b * inv,
            c: -self.c * inv,
            d: self.a * inv,
            e: (self.c * self.f - self.d * self.e) * inv,
            f: (self.b * self.e - self.a * self.f) * inv,
        })
    }

    /// The axis-aligned bounding box of a rect mapped through the transform.
    /// For rotations this is strictly larger than the rotated shape — that is
    /// what a damage region needs (cover everything that changed), not what a
    /// hit test needs.
    pub fn map_rect(self, r: Rect) -> Rect {
        let corners = [
            self.map_point(Point::new(r.x0, r.y0)),
            self.map_point(Point::new(r.x1, r.y0)),
            self.map_point(Point::new(r.x0, r.y1)),
            self.map_point(Point::new(r.x1, r.y1)),
        ];
        let (mut x0, mut y0, mut x1, mut y1) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for p in corners {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        }
        Rect { x0, y0, x1, y1 }
    }

    #[inline]
    pub fn is_identity(self) -> bool {
        self == Self::IDENTITY
    }

    #[inline]
    pub fn map_point(self, p: Point) -> Point {
        Point::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_defaults_are_deterministic() {
        assert_eq!(Point::default(), Point::ZERO);
        assert_eq!(Size::default(), Size::ZERO);
        assert_eq!(Rect::default(), Rect::ZERO);
        assert_eq!(Insets::default(), Insets::ZERO);
        assert_eq!(Transform::default(), Transform::IDENTITY);
        assert!(Constraints::default().max.width.is_infinite());
    }

    #[test]
    fn rect_from_origin_size_and_contains() {
        let r = Rect::from_origin_size(Point::new(10.0, 20.0), Size::new(30.0, 40.0));
        assert_eq!(r.width(), 30.0);
        assert_eq!(r.height(), 40.0);
        assert!(r.contains(Point::new(10.0, 20.0)));
        assert!(!r.contains(Point::new(40.0, 20.0)));
    }

    #[test]
    fn constraints_clamp() {
        let c = Constraints {
            min: Size::new(10.0, 10.0),
            max: Size::new(100.0, 50.0),
        };
        assert_eq!(c.constrain(Size::new(5.0, 200.0)), Size::new(10.0, 50.0));
    }

    #[test]
    fn transform_translate_maps_point() {
        let t = Transform::translate(3.0, 4.0);
        assert_eq!(t.map_point(Point::new(1.0, 2.0)), Point::new(4.0, 6.0));
    }

    #[test]
    fn transform_scale_maps_about_origin_and_about_a_point() {
        let s = Transform::scale(2.0, 3.0);
        assert_eq!(s.map_point(Point::new(1.0, 1.0)), Point::new(2.0, 3.0));

        // Scaling around (10, 10) must leave the pivot fixed.
        let around = Transform::scale_around(2.0, 2.0, Point::new(10.0, 10.0));
        assert_eq!(
            around.map_point(Point::new(10.0, 10.0)),
            Point::new(10.0, 10.0)
        );
        // And push a point 5 to the left of the pivot 5 further away.
        assert_eq!(
            around.map_point(Point::new(5.0, 10.0)),
            Point::new(0.0, 10.0)
        );
    }

    #[test]
    fn transform_rotate_spins_the_axes() {
        // A quarter turn clockwise (y down) maps +x onto +y.
        let r = Transform::rotate(std::f64::consts::FRAC_PI_2);
        let mapped = r.map_point(Point::new(1.0, 0.0));
        assert!(mapped.x.abs() < 1e-9 && (mapped.y - 1.0).abs() < 1e-9);

        // Rotating around the pivot leaves the pivot fixed.
        let around = Transform::rotate_around(std::f64::consts::FRAC_PI_2, Point::new(4.0, 4.0));
        assert_eq!(around.map_point(Point::new(4.0, 4.0)), Point::new(4.0, 4.0));
    }

    #[test]
    fn transform_multiply_applies_the_right_one_first() {
        let t = Transform::translate(10.0, 0.0);
        let s = Transform::scale(2.0, 2.0);
        // translate ∘ scale: scale first, then shift.
        let both = t.multiply(s);
        assert_eq!(both.map_point(Point::new(1.0, 0.0)), Point::new(12.0, 0.0));
        // The other order shifts first, so the scale doubles the shift too.
        let swapped = s.multiply(t);
        assert_eq!(
            swapped.map_point(Point::new(1.0, 0.0)),
            Point::new(22.0, 0.0)
        );
    }

    #[test]
    fn transform_inverse_undoes_the_mapping_and_rejects_singular() {
        let t = Transform::rotate_around(0.7, Point::new(3.0, 5.0))
            .multiply(Transform::scale_around(2.0, 3.0, Point::new(1.0, 1.0)))
            .multiply(Transform::translate(4.0, -2.0));
        let inv = t
            .inverse()
            .expect("a non-singular transform has an inverse");
        let p = Point::new(7.25, -3.5);
        let back = inv.map_point(t.map_point(p));
        assert!((back.x - p.x).abs() < 1e-9 && (back.y - p.y).abs() < 1e-9);

        // A zero scale collapses the plane: nothing maps back.
        assert!(Transform::scale(0.0, 1.0).inverse().is_none());
    }

    #[test]
    fn transform_map_rect_is_the_bounding_box_of_the_mapped_corners() {
        // Translation maps the rect exactly (no rotation slop).
        let t = Transform::translate(5.0, 7.0);
        let r = Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(4.0, 2.0));
        assert_eq!(
            t.map_rect(r),
            Rect {
                x0: 5.0,
                y0: 7.0,
                x1: 9.0,
                y1: 9.0
            }
        );

        // A quarter turn about the origin maps [0..4]×[0..2] to a box on the
        // other side; the bbox must cover every mapped corner (within fp slop).
        let t90 = Transform::rotate(std::f64::consts::FRAC_PI_2);
        let r90 = t90.map_rect(r);
        for corner in [
            Point::new(0.0, 0.0),
            Point::new(4.0, 2.0),
            Point::new(0.0, 2.0),
            Point::new(4.0, 0.0),
        ] {
            let c = t90.map_point(corner);
            assert!(
                c.x >= r90.x0 - 1e-9
                    && c.x <= r90.x1 + 1e-9
                    && c.y >= r90.y0 - 1e-9
                    && c.y <= r90.y1 + 1e-9,
                "the bbox covers the mapped corner {c:?}: {r90:?}"
            );
        }
    }
}
