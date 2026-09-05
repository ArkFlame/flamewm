#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(self) -> i32 {
        self.x
            .saturating_add(i32::try_from(self.width).unwrap_or(i32::MAX))
    }

    pub fn bottom(self) -> i32 {
        self.y
            .saturating_add(i32::try_from(self.height).unwrap_or(i32::MAX))
    }

    pub fn clamp_inside(self, bounds: Self) -> Self {
        let width = self.width.min(bounds.width.max(1));
        let height = self.height.min(bounds.height.max(1));
        let max_x = bounds
            .right()
            .saturating_sub(i32::try_from(width).unwrap_or(i32::MAX));
        let max_y = bounds
            .bottom()
            .saturating_sub(i32::try_from(height).unwrap_or(i32::MAX));
        Self {
            x: self.x.clamp(bounds.x, max_x.max(bounds.x)),
            y: self.y.clamp(bounds.y, max_y.max(bounds.y)),
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapTarget {
    None,
    LeftHalf,
    RightHalf,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Maximize,
}

pub fn snap_target(pointer_x: i32, pointer_y: i32, area: Rect, zone: i32) -> SnapTarget {
    let zone = zone.max(1);
    let left = pointer_x <= area.x.saturating_add(zone);
    let right = pointer_x >= area.right().saturating_sub(zone);
    let top = pointer_y <= area.y.saturating_add(zone);
    let bottom = pointer_y >= area.bottom().saturating_sub(zone);
    if left && top {
        SnapTarget::TopLeft
    } else if right && top {
        SnapTarget::TopRight
    } else if left && bottom {
        SnapTarget::BottomLeft
    } else if right && bottom {
        SnapTarget::BottomRight
    } else if top {
        SnapTarget::Maximize
    } else if left {
        SnapTarget::LeftHalf
    } else if right {
        SnapTarget::RightHalf
    } else {
        SnapTarget::None
    }
}

pub fn snap_geometry(target: SnapTarget, area: Rect) -> Rect {
    let left_width = area.width / 2;
    let right_width = area.width.saturating_sub(left_width);
    let top_height = area.height / 2;
    let bottom_height = area.height.saturating_sub(top_height);
    let right_x = area
        .x
        .saturating_add(i32::try_from(left_width).unwrap_or(i32::MAX));
    let bottom_y = area
        .y
        .saturating_add(i32::try_from(top_height).unwrap_or(i32::MAX));
    match target {
        SnapTarget::None => area,
        SnapTarget::LeftHalf => Rect::new(area.x, area.y, left_width, area.height),
        SnapTarget::RightHalf => Rect::new(right_x, area.y, right_width, area.height),
        SnapTarget::TopLeft => Rect::new(area.x, area.y, left_width, top_height),
        SnapTarget::TopRight => Rect::new(right_x, area.y, right_width, top_height),
        SnapTarget::BottomLeft => Rect::new(area.x, bottom_y, left_width, bottom_height),
        SnapTarget::BottomRight => Rect::new(right_x, bottom_y, right_width, bottom_height),
        SnapTarget::Maximize => area,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_half_preserves_odd_pixel() {
        assert_eq!(
            snap_geometry(SnapTarget::RightHalf, Rect::new(-100, 20, 1001, 701)),
            Rect::new(400, 20, 501, 701),
        );
    }

    #[test]
    fn corner_wins_over_side() {
        assert_eq!(
            snap_target(2, 2, Rect::new(0, 0, 1920, 1080), 24),
            SnapTarget::TopLeft,
        );
    }

    #[test]
    fn clamp_handles_negative_monitor_origin() {
        let bounds = Rect::new(-1920, 0, 1920, 1080);
        assert_eq!(
            Rect::new(-2500, -100, 800, 600).clamp_inside(bounds),
            Rect::new(-1920, 0, 800, 600)
        );
    }
}
