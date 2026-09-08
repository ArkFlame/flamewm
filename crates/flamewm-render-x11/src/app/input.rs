//! Native input helpers: PointerButton mapping (local duplicate, no
//! feature logic), wheel Button4/5 -> scroll delta, slider thumb/track
//! geometry for range drag.

use super::*;

/// Local raw-X-button mapping. Mirrors `PointerButton::from_raw_x` without
/// importing feature logic; wheel buttons never act as press/release.
pub(crate) fn pointer_button_from_raw(button: u32) -> PointerButton {
    PointerButton::from_raw_x(button)
}

/// Wheel Button4/5 -> vertical scroll delta (Up = -1 line, Down = +1).
/// Returns None for non-wheel buttons.
pub(crate) fn wheel_scroll_delta(button: PointerButton) -> Option<(f32, f32)> {
    const LINE: f32 = 24.0;
    match button {
        PointerButton::WheelUp => Some((0.0, -LINE)),
        PointerButton::WheelDown => Some((0.0, LINE)),
        _ => None,
    }
}

/// Horizontal slider track/thumb geometry for pointer-drag mapping.
/// `track` is the full track rect (device px), `value/min/max` the range;
/// thumb width is 12px clamped to the track.
pub(crate) fn slider_thumb_rect(track: Rect, min: f32, max: f32, value: f32) -> Rect {
    let span = (max - min).max(f32::EPSILON);
    let ratio = ((value - min) / span).clamp(0.0, 1.0);
    let thumb_w = 12.0f32.min(track.width.max(1.0));
    let travel = (track.width - thumb_w).max(0.0);
    Rect {
        x: track.x + ratio * travel,
        y: track.y,
        width: thumb_w,
        height: track.height,
    }
}

/// Map a pointer x on the track to a snapped value in [min, max].
/// `step <= 0` means continuous.
pub(crate) fn slider_value_from_pointer(
    track: Rect,
    min: f32,
    max: f32,
    step: f32,
    pointer_x: f32,
) -> f32 {
    let (lo, hi) = if min <= max { (min, max) } else { (max, min) };
    let thumb_w = 12.0f32.min(track.width.max(1.0));
    let travel = (track.width - thumb_w).max(0.0);
    if travel <= 0.0 {
        return lo;
    }
    let ratio = ((pointer_x - track.x - thumb_w / 2.0) / travel).clamp(0.0, 1.0);
    let raw = lo + ratio * (hi - lo);
    if step <= 0.0 {
        raw
    } else {
        let steps = ((raw - lo) / step).round();
        (lo + steps * step).clamp(lo, hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_maps_to_scroll_delta_only() {
        assert_eq!(
            wheel_scroll_delta(pointer_button_from_raw(4)),
            Some((0.0, -24.0))
        );
        assert_eq!(
            wheel_scroll_delta(pointer_button_from_raw(5)),
            Some((0.0, 24.0))
        );
        assert_eq!(wheel_scroll_delta(pointer_button_from_raw(1)), None);
    }

    #[test]
    fn slider_pointer_round_trips() {
        let track = Rect {
            x: 10.0,
            y: 0.0,
            width: 100.0,
            height: 8.0,
        };
        let thumb = slider_thumb_rect(track, 0.0, 100.0, 50.0);
        assert!((thumb.x - (10.0 + 44.0)).abs() < 0.5);
        let v = slider_value_from_pointer(track, 0.0, 100.0, 0.0, thumb.x + 6.0);
        assert!((v - 50.0).abs() < 1.0);
    }
}
