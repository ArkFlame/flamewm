use core::cmp::{max, min};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

impl Size {
    #[must_use]
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[must_use]
    pub const fn from_parts(origin: Point, size: Size) -> Self {
        Self::new(origin.x, origin.y, size.width, size.height)
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.width > 0 && self.height > 0
    }

    #[must_use]
    pub const fn right(self) -> i32 {
        self.x + self.width
    }

    #[must_use]
    pub const fn bottom(self) -> i32 {
        self.y + self.height
    }

    #[must_use]
    pub const fn origin(self) -> Point {
        Point::new(self.x, self.y)
    }

    #[must_use]
    pub const fn size(self) -> Size {
        Size::new(self.width, self.height)
    }

    #[must_use]
    pub const fn contains(self, point: Point) -> bool {
        point.x >= self.x && point.x < self.right() && point.y >= self.y && point.y < self.bottom()
    }

    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    #[must_use]
    pub fn intersection(self, other: Self) -> Option<Self> {
        let x = max(self.x, other.x);
        let y = max(self.y, other.y);
        let right = min(self.right(), other.right());
        let bottom = min(self.bottom(), other.bottom());
        let width = right - x;
        let height = bottom - y;
        (width > 0 && height > 0).then(|| Self::new(x, y, width, height))
    }

    #[must_use]
    pub fn union(self, other: Self) -> Self {
        if !self.is_valid() {
            return other;
        }
        if !other.is_valid() {
            return self;
        }
        let x = min(self.x, other.x);
        let y = min(self.y, other.y);
        let right = max(self.right(), other.right());
        let bottom = max(self.bottom(), other.bottom());
        Self::new(x, y, right - x, bottom - y)
    }

    /// Keep the rectangle inside `bounds`, shrinking only when it cannot fit.
    #[must_use]
    pub fn clamp_inside(self, bounds: Self) -> Self {
        if !bounds.is_valid() {
            return Self::default();
        }
        let width = self.width.max(0).min(bounds.width);
        let height = self.height.max(0).min(bounds.height);
        let max_x = bounds.right() - width;
        let max_y = bounds.bottom() - height;
        Self::new(
            self.x.clamp(bounds.x, max_x),
            self.y.clamp(bounds.y, max_y),
            width,
            height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_inside_handles_negative_monitor_origin() {
        let bounds = Rect::new(-1920, 0, 1920, 1080);
        let rect = Rect::new(-2200, 900, 900, 400);
        assert_eq!(rect.clamp_inside(bounds), Rect::new(-1920, 680, 900, 400));
    }
}
