//! Pure popover placement: pick an on-screen origin for a popup of
//! `popup_size` anchored to `source_rect`, preferring `edge`, clamped to
//! `work_area` with `offset` gap. No rendering here.

use flamewm_api::{Point, Rect, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopoverEdge {
    Below,
    Above,
    Right,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopoverGeometry {
    pub origin: Point,
    pub edge: PopoverEdge,
}

impl PopoverGeometry {
    #[must_use]
    pub fn place(
        source_rect: Rect,
        popup_size: Size,
        edge: PopoverEdge,
        work_area: Rect,
        offset: i32,
    ) -> Self {
        let size = Size::new(popup_size.width.max(0), popup_size.height.max(0));
        let candidates = [
            edge,
            Self::opposite(edge),
            PopoverEdge::Below,
            PopoverEdge::Above,
        ];
        for candidate in candidates {
            let origin = Self::origin_for(source_rect, size, candidate, offset);
            if Self::fits(origin, size, work_area) {
                return Self {
                    origin: Self::clamp_to_work_area(origin, size, work_area),
                    edge: candidate,
                };
            }
        }
        // Nothing fits: clamp preferred origin into the work area.
        let origin = Self::clamp_to_work_area(
            Self::origin_for(source_rect, size, edge, offset),
            size,
            work_area,
        );
        Self { origin, edge }
    }

    const fn opposite(edge: PopoverEdge) -> PopoverEdge {
        match edge {
            PopoverEdge::Below => PopoverEdge::Above,
            PopoverEdge::Above => PopoverEdge::Below,
            PopoverEdge::Right => PopoverEdge::Left,
            PopoverEdge::Left => PopoverEdge::Right,
        }
    }

    fn origin_for(source: Rect, size: Size, edge: PopoverEdge, offset: i32) -> Point {
        match edge {
            PopoverEdge::Below => Point::new(source.x, source.bottom() + offset),
            PopoverEdge::Above => Point::new(source.x, source.y - size.height - offset),
            PopoverEdge::Right => Point::new(source.right() + offset, source.y),
            PopoverEdge::Left => Point::new(source.x - size.width - offset, source.y),
        }
    }

    fn fits(origin: Point, size: Size, work_area: Rect) -> bool {
        let rect = Rect::from_parts(origin, size);
        origin.x >= work_area.x
            && origin.y >= work_area.y
            && rect.right() <= work_area.right()
            && rect.bottom() <= work_area.bottom()
    }

    fn clamp_to_work_area(origin: Point, size: Size, work_area: Rect) -> Point {
        Rect::from_parts(origin, size)
            .clamp_inside(work_area)
            .origin()
    }
}
