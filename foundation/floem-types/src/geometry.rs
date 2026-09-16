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
}
