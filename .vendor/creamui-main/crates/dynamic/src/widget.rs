//! Widget construction — the dlopen-backed equivalent of `creamui_widgets`.

use crate::runtime::Runtime;
use crate::value::{Color, Style, Theme};
use crate::window::Context;
use std::ffi::{c_void, CStr, CString};
use std::os::raw::{c_char, c_int};
use std::rc::Rc;

/// An opaque, not-yet-attached (or already-attached) widget subtree.
///
/// Consumed by [`Widget::child`] (attaches to a parent) or by returning it
/// from a `build_ui` closure (attaches it as the window's root) — either
/// way, ownership crosses into the C ABI and this wrapper's `Drop` becomes
/// a no-op. If neither happens, `Drop` frees it.
pub struct Widget {
    rt: Rc<Runtime>,
    ptr: *mut c_void,
    consumed: std::cell::Cell<bool>,
}

impl Widget {
    fn new(rt: &Rc<Runtime>, ptr: *mut c_void) -> Self {
        Widget {
            rt: rt.clone(),
            ptr,
            consumed: std::cell::Cell::new(false),
        }
    }

    pub(crate) fn into_raw(self) -> *mut c_void {
        self.consumed.set(true);
        self.ptr
    }

    /// Sets a block's background color. No-op on any widget kind other than
    /// one created by [`block`]/[`block_styled`].
    pub fn background(self, color: Color) -> Self {
        unsafe { (self.rt.sym.block_set_background)(self.ptr, color) };
        self
    }

    /// Sets a block's corner radius, in logical pixels. Same widget-kind
    /// restriction as [`Widget::background`].
    pub fn corner_radius(self, radius: f32) -> Self {
        unsafe { (self.rt.sym.block_set_corner_radius)(self.ptr, radius) };
        self
    }

    /// Attaches `child`, taking ownership of it. Works for widgets created
    /// by [`block`]/[`block_styled`]/[`scroll_view`].
    pub fn child(self, child: Widget) -> Self {
        let child_ptr = child.into_raw();
        unsafe { (self.rt.sym.block_add_child)(self.ptr, child_ptr) };
        self
    }

    /// Sets the placeholder text (and its themed disabled-text color) shown
    /// when a text input's value is empty. No-op on any other widget kind.
    pub fn placeholder(self, theme: Theme, text: &str) -> Self {
        let c_text = CString::new(text).unwrap_or_default();
        unsafe { (self.rt.sym.text_input_set_placeholder)(theme, self.ptr, c_text.as_ptr()) };
        self
    }

    /// Sets a text area's placeholder. No-op on widgets of another kind.
    pub fn area_placeholder(self, theme: Theme, text: &str) -> Self {
        let c_text = CString::new(text).unwrap_or_default();
        unsafe { (self.rt.sym.text_area_set_placeholder)(theme, self.ptr, c_text.as_ptr()) };
        self
    }

    /// Sets the background and foreground design tokens for selected text in
    /// a dynamically-created textarea. No-op for other widget kinds.
    pub fn area_selection_colors(self, background: Color, text: Color) -> Self {
        unsafe { (self.rt.sym.text_area_set_selection_colors)(self.ptr, background, text) };
        self
    }
}

impl Drop for Widget {
    fn drop(&mut self) {
        if !self.consumed.get() {
            unsafe { (self.rt.sym.widget_free)(self.ptr) };
        }
    }
}

/// A semantic block container. For full layout control, use [`block_styled`].
pub fn block(ctx: &Context) -> Widget {
    Widget::new(&ctx.rt, unsafe { (ctx.rt.sym.block_new)() })
}

/// A block container with caller-supplied layout properties.
pub fn block_styled(ctx: &Context, style: Style) -> Widget {
    Widget::new(&ctx.rt, unsafe { (ctx.rt.sym.block_new_styled)(style) })
}

/// An unthemed, single-line text label with an explicit color and size.
pub fn text(ctx: &Context, text: &str, color: Color, font_size: f32) -> Widget {
    let c_text = CString::new(text).unwrap_or_default();
    Widget::new(&ctx.rt, unsafe {
        (ctx.rt.sym.text_new)(c_text.as_ptr(), color, font_size)
    })
}

/// A themed text label using `theme`'s primary text color.
pub fn themed_text(ctx: &Context, theme: Theme, text: &str) -> Widget {
    let c_text = CString::new(text).unwrap_or_default();
    Widget::new(&ctx.rt, unsafe {
        (ctx.rt.sym.themed_text_new)(theme, c_text.as_ptr())
    })
}

/// A themed text label using `theme`'s secondary (muted) text color — e.g.
/// for captions or de-emphasized helper text.
pub fn themed_text_secondary(ctx: &Context, theme: Theme, text: &str) -> Widget {
    let c_text = CString::new(text).unwrap_or_default();
    Widget::new(&ctx.rt, unsafe {
        (ctx.rt.sym.themed_text_secondary_new)(theme, c_text.as_ptr())
    })
}

/// Same as [`themed_text`], but with an explicit font size in logical
/// pixels instead of the default 14.0.
pub fn themed_text_sized(ctx: &Context, theme: Theme, text: &str, font_size: f32) -> Widget {
    let c_text = CString::new(text).unwrap_or_default();
    Widget::new(&ctx.rt, unsafe {
        (ctx.rt.sym.themed_text_new_sized)(theme, c_text.as_ptr(), font_size)
    })
}

extern "C" fn click_trampoline<F: FnMut() + 'static>(userdata: *mut c_void) {
    let f = unsafe { &mut *(userdata as *mut F) };
    f();
}

extern "C" fn text_change_trampoline<F: FnMut(String) + 'static>(
    value: *const c_char,
    userdata: *mut c_void,
) {
    let f = unsafe { &mut *(userdata as *mut F) };
    let value = if value.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    };
    f(value);
}

extern "C" fn float_change_trampoline<F: FnMut(f32) + 'static>(value: f32, userdata: *mut c_void) {
    let f = unsafe { &mut *(userdata as *mut F) };
    f(value);
}

/// A themed button labeled `text`, calling `on_click` on every click.
///
/// `on_click` is boxed fresh every time the enclosing `build_ui` runs (a
/// new widget tree — and new callback closures for it — is built every
/// repaint); [`Context`]'s closure arena keeps it alive for exactly as long
/// as it might still be invoked, then frees it. See [`crate::arena`].
pub fn button<F>(ctx: &Context, theme: Theme, text: &str, on_click: F) -> Widget
where
    F: FnMut() + 'static,
{
    let c_text = CString::new(text).unwrap_or_default();
    let userdata = ctx.arena.keep(on_click) as *mut c_void;
    Widget::new(&ctx.rt, unsafe {
        (ctx.rt.sym.button_new)(theme, c_text.as_ptr(), click_trampoline::<F>, userdata)
    })
}

/// A themed checkbox, calling `on_click` on every click (same
/// "caller owns the checked state" pattern as the native `Checkbox`: toggle
/// your own state in `on_click` and pass the new value back in on the next
/// `build_ui` call).
pub fn checkbox<F>(ctx: &Context, theme: Theme, checked: bool, on_click: F) -> Widget
where
    F: FnMut() + 'static,
{
    let userdata = ctx.arena.keep(on_click) as *mut c_void;
    Widget::new(&ctx.rt, unsafe {
        (ctx.rt.sym.checkbox_new)(theme, checked as c_int, click_trampoline::<F>, userdata)
    })
}

/// A themed single-line text input with a caller-supplied [`Style`] (e.g.
/// its width/height). `on_change` fires on every keystroke with the new
/// value.
pub fn text_input<F>(ctx: &Context, theme: Theme, style: Style, value: &str, on_change: F) -> Widget
where
    F: FnMut(String) + 'static,
{
    let c_value = CString::new(value).unwrap_or_default();
    let userdata = ctx.arena.keep(on_change) as *mut c_void;
    Widget::new(&ctx.rt, unsafe {
        (ctx.rt.sym.text_input_new)(
            theme,
            style,
            c_value.as_ptr(),
            text_change_trampoline::<F>,
            userdata,
        )
    })
}

/// A themed multi-line text editor, constructed over the dynamic ABI.
pub fn text_area<F>(ctx: &Context, theme: Theme, style: Style, value: &str, on_change: F) -> Widget
where
    F: FnMut(String) + 'static,
{
    let c_value = CString::new(value).unwrap_or_default();
    let userdata = ctx.arena.keep(on_change) as *mut c_void;
    Widget::new(&ctx.rt, unsafe {
        (ctx.rt.sym.text_area_new)(
            theme,
            style,
            c_value.as_ptr(),
            text_change_trampoline::<F>,
            userdata,
        )
    })
}

/// A themed horizontal slider with a caller-supplied [`Style`]. `on_change`
/// fires with the new `0.0..=1.0` value as the handle is dragged.
pub fn slider<F>(ctx: &Context, theme: Theme, style: Style, value: f32, on_change: F) -> Widget
where
    F: FnMut(f32) + 'static,
{
    let userdata = ctx.arena.keep(on_change) as *mut c_void;
    Widget::new(&ctx.rt, unsafe {
        (ctx.rt.sym.slider_new)(theme, style, value, float_change_trampoline::<F>, userdata)
    })
}

/// A themed vertically-scrollable container with a caller-supplied
/// [`Style`] (typically a fixed `width`/`height` viewport). Attach children
/// with [`Widget::child`]. `on_scroll` fires with the wheel delta on every
/// wheel event over the view; the caller owns and clamps the scroll offset,
/// same as the native `ScrollView`.
pub fn scroll_view<F>(
    ctx: &Context,
    theme: Theme,
    style: Style,
    scroll_y: f32,
    on_scroll: F,
) -> Widget
where
    F: FnMut(f32) + 'static,
{
    let userdata = ctx.arena.keep(on_scroll) as *mut c_void;
    Widget::new(&ctx.rt, unsafe {
        (ctx.rt.sym.scroll_view_new)(
            theme,
            style,
            scroll_y,
            float_change_trampoline::<F>,
            userdata,
        )
    })
}
