use x11rb::connection::Connection;
use x11rb::errors::ReplyError;
use x11rb::protocol::xproto::*;

pub const TITLEBAR_HEIGHT: u16 = 32;
pub const FRAME_BORDER: u16 = 1;
pub const CLOSE_HOVER_RADIUS: u16 = 6;

#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub titlebar_height: u16,
    pub frame_border: u16,
}

impl Metrics {
    pub const fn new(titlebar_height: u16, frame_border: u16) -> Self {
        Self {
            titlebar_height,
            frame_border,
        }
    }
}

pub fn draw<C: Connection>(
    conn: &C,
    gc: Gcontext,
    frame: Window,
    width: u16,
    title: &str,
    metrics: Metrics,
    close_hover: bool,
    screen: &Screen,
) -> Result<(), ReplyError> {
    let title_height = metrics.titlebar_height;
    conn.change_gc(gc, &ChangeGCAux::new().foreground(screen.black_pixel))?;
    conn.poly_fill_rectangle(
        frame,
        gc,
        &[Rectangle {
            x: 0,
            y: 0,
            width,
            height: title_height,
        }],
    )?;
    conn.change_gc(gc, &ChangeGCAux::new().foreground(screen.white_pixel))?;
    let title = &title.as_bytes()[..title.len().min(96)];
    conn.image_text8(frame, gc, 12, 21, title)?;

    let button = i16::try_from(title_height).unwrap_or(TITLEBAR_HEIGHT as i16);
    let right = i16::try_from(width).unwrap_or(i16::MAX);
    for offset in [button, button * 2, button * 3] {
        let x = right.saturating_sub(offset);
        conn.poly_line(
            CoordMode::ORIGIN,
            frame,
            gc,
            &[Point { x, y: 0 }, Point { x, y: button }],
        )?;
    }
    let close_left = right.saturating_sub(button);
    if close_hover {
        let radius = i16::try_from(CLOSE_HOVER_RADIUS.min(title_height / 2)).unwrap_or(6);
        conn.poly_fill_arc(
            frame,
            gc,
            &[Arc {
                x: right - radius * 2,
                y: 0,
                width: (radius * 2) as u16,
                height: (radius * 2) as u16,
                angle1: 0,
                angle2: 360_i16 * 64,
            }],
        )?;
    }
    let inset = 10_i16.min(button / 3);
    conn.poly_line(
        CoordMode::ORIGIN,
        frame,
        gc,
        &[
            Point {
                x: close_left + inset,
                y: inset,
            },
            Point {
                x: right - inset,
                y: button - inset,
            },
        ],
    )?;
    conn.poly_line(
        CoordMode::ORIGIN,
        frame,
        gc,
        &[
            Point {
                x: close_left + inset,
                y: button - inset,
            },
            Point {
                x: right - inset,
                y: inset,
            },
        ],
    )?;
    Ok(())
}
