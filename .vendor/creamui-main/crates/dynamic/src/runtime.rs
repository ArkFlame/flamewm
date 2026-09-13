//! Loads the `creamui` `cdylib` and resolves every C-ABI symbol this crate
//! calls into, exactly once, at startup — this is the one place `unsafe`
//! `dlopen`/`dlsym` calls happen. Everything built on top ([`crate::Widget`],
//! [`crate::SignalI32`] and friends, [`crate::run`]/[`crate::AppBuilder`])
//! is a safe wrapper around a resolved, verified-present function pointer.

use creamui_abi::{CColor, CStyle, CTheme, CWindowOptions};
use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::fmt;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub(crate) type BlockNewFn = unsafe extern "C" fn() -> *mut c_void;
pub(crate) type BlockNewStyledFn = unsafe extern "C" fn(CStyle) -> *mut c_void;
pub(crate) type BlockSetBackgroundFn = unsafe extern "C" fn(*mut c_void, CColor);
pub(crate) type BlockSetCornerRadiusFn = unsafe extern "C" fn(*mut c_void, f32);
pub(crate) type BlockAddChildFn = unsafe extern "C" fn(*mut c_void, *mut c_void);
pub(crate) type ThemeFn = unsafe extern "C" fn() -> CTheme;
pub(crate) type TextNewFn = unsafe extern "C" fn(*const c_char, CColor, f32) -> *mut c_void;
pub(crate) type ThemedTextNewFn = unsafe extern "C" fn(CTheme, *const c_char) -> *mut c_void;
pub(crate) type ThemedTextNewSizedFn =
    unsafe extern "C" fn(CTheme, *const c_char, f32) -> *mut c_void;
pub(crate) type ButtonNewFn = unsafe extern "C" fn(
    CTheme,
    *const c_char,
    extern "C" fn(*mut c_void),
    *mut c_void,
) -> *mut c_void;
pub(crate) type CheckboxNewFn =
    unsafe extern "C" fn(CTheme, c_int, extern "C" fn(*mut c_void), *mut c_void) -> *mut c_void;
pub(crate) type TextInputNewFn = unsafe extern "C" fn(
    CTheme,
    CStyle,
    *const c_char,
    extern "C" fn(*const c_char, *mut c_void),
    *mut c_void,
) -> *mut c_void;
pub(crate) type TextInputSetPlaceholderFn =
    unsafe extern "C" fn(CTheme, *mut c_void, *const c_char);
pub(crate) type TextAreaNewFn = TextInputNewFn;
pub(crate) type TextAreaSetPlaceholderFn = TextInputSetPlaceholderFn;
pub(crate) type TextAreaSetSelectionColorsFn = unsafe extern "C" fn(*mut c_void, CColor, CColor);
pub(crate) type SliderNewFn = unsafe extern "C" fn(
    CTheme,
    CStyle,
    f32,
    extern "C" fn(f32, *mut c_void),
    *mut c_void,
) -> *mut c_void;
pub(crate) type ScrollViewNewFn = unsafe extern "C" fn(
    CTheme,
    CStyle,
    f32,
    extern "C" fn(f32, *mut c_void),
    *mut c_void,
) -> *mut c_void;
pub(crate) type WidgetFreeFn = unsafe extern "C" fn(*mut c_void);

pub(crate) type SignalI32NewFn = unsafe extern "C" fn(i32) -> *mut c_void;
pub(crate) type SignalI32GetFn = unsafe extern "C" fn(*const c_void) -> i32;
pub(crate) type SignalI32SetFn = unsafe extern "C" fn(*const c_void, i32);
pub(crate) type SignalI32FreeFn = unsafe extern "C" fn(*mut c_void);
pub(crate) type SignalF32NewFn = unsafe extern "C" fn(f32) -> *mut c_void;
pub(crate) type SignalF32GetFn = unsafe extern "C" fn(*const c_void) -> f32;
pub(crate) type SignalF32SetFn = unsafe extern "C" fn(*const c_void, f32);
pub(crate) type SignalF32FreeFn = unsafe extern "C" fn(*mut c_void);
pub(crate) type SignalStringNewFn = unsafe extern "C" fn(*const c_char) -> *mut c_void;
pub(crate) type SignalStringGetFn = unsafe extern "C" fn(*const c_void) -> *const c_char;
pub(crate) type SignalStringSetFn = unsafe extern "C" fn(*const c_void, *const c_char);
pub(crate) type SignalStringFreeFn = unsafe extern "C" fn(*mut c_void);

pub(crate) type WindowResizeFn = unsafe extern "C" fn(*const c_void, u32, u32);
pub(crate) type WindowSetPositionFn = unsafe extern "C" fn(*const c_void, i32, i32);
pub(crate) type WindowSetAlwaysOnTopFn = unsafe extern "C" fn(*const c_void, c_int);
pub(crate) type WindowHandleFreeFn = unsafe extern "C" fn(*mut c_void);

pub(crate) type BuildFn = extern "C" fn(f32, f32, *mut c_void) -> *mut c_void;
pub(crate) type WindowReadyFn = extern "C" fn(*mut c_void, *mut c_void);
pub(crate) type AppBuilderNewFn = unsafe extern "C" fn() -> *mut c_void;
pub(crate) type AppBuilderAddWindowFn = unsafe extern "C" fn(
    *mut c_void,
    CWindowOptions,
    CColor,
    BuildFn,
    Option<WindowReadyFn>,
    *mut c_void,
);
pub(crate) type AppBuilderRunFn = unsafe extern "C" fn(*mut c_void);

/// Every resolved symbol this crate needs. Private — callers only ever see
/// [`Runtime`]'s safe methods and the free functions in [`crate::widget`]/
/// [`crate::signal`] that take a [`Runtime`]/[`crate::Context`].
pub(crate) struct Symbols {
    pub(crate) block_new: BlockNewFn,
    pub(crate) block_new_styled: BlockNewStyledFn,
    pub(crate) block_set_background: BlockSetBackgroundFn,
    pub(crate) block_set_corner_radius: BlockSetCornerRadiusFn,
    pub(crate) block_add_child: BlockAddChildFn,
    pub(crate) theme_dark: ThemeFn,
    pub(crate) theme_light: ThemeFn,
    pub(crate) text_new: TextNewFn,
    pub(crate) themed_text_new: ThemedTextNewFn,
    pub(crate) themed_text_secondary_new: ThemedTextNewFn,
    pub(crate) themed_text_new_sized: ThemedTextNewSizedFn,
    pub(crate) button_new: ButtonNewFn,
    pub(crate) checkbox_new: CheckboxNewFn,
    pub(crate) text_input_new: TextInputNewFn,
    pub(crate) text_input_set_placeholder: TextInputSetPlaceholderFn,
    pub(crate) text_area_new: TextAreaNewFn,
    pub(crate) text_area_set_placeholder: TextAreaSetPlaceholderFn,
    pub(crate) text_area_set_selection_colors: TextAreaSetSelectionColorsFn,
    pub(crate) slider_new: SliderNewFn,
    pub(crate) scroll_view_new: ScrollViewNewFn,
    pub(crate) widget_free: WidgetFreeFn,
    pub(crate) signal_i32_new: SignalI32NewFn,
    pub(crate) signal_i32_get: SignalI32GetFn,
    pub(crate) signal_i32_set: SignalI32SetFn,
    pub(crate) signal_i32_free: SignalI32FreeFn,
    pub(crate) signal_f32_new: SignalF32NewFn,
    pub(crate) signal_f32_get: SignalF32GetFn,
    pub(crate) signal_f32_set: SignalF32SetFn,
    pub(crate) signal_f32_free: SignalF32FreeFn,
    pub(crate) signal_string_new: SignalStringNewFn,
    pub(crate) signal_string_get: SignalStringGetFn,
    pub(crate) signal_string_set: SignalStringSetFn,
    pub(crate) signal_string_free: SignalStringFreeFn,
    pub(crate) window_resize: WindowResizeFn,
    pub(crate) window_set_position: WindowSetPositionFn,
    pub(crate) window_set_always_on_top: WindowSetAlwaysOnTopFn,
    pub(crate) window_handle_free: WindowHandleFreeFn,
    pub(crate) app_builder_new: AppBuilderNewFn,
    pub(crate) app_builder_add_window: AppBuilderAddWindowFn,
    pub(crate) app_builder_run: AppBuilderRunFn,
}

/// Failure to load the `cdylib` or resolve one of its expected symbols.
#[derive(Debug)]
pub enum LoadError {
    Load(libloading::Error),
    MissingSymbol(&'static str, libloading::Error),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Load(e) => write!(f, "failed to load library: {e}"),
            LoadError::MissingSymbol(name, e) => write!(f, "missing symbol {name:?}: {e}"),
        }
    }
}

impl std::error::Error for LoadError {}

macro_rules! resolve {
    ($lib:expr, $name:literal) => {{
        let sym: Symbol<_> = $lib
            .get($name.as_bytes())
            .map_err(|e| LoadError::MissingSymbol($name, e))?;
        *sym
    }};
}

impl Symbols {
    fn load(lib: &Library) -> Result<Self, LoadError> {
        // SAFETY: every symbol below is resolved by name against the
        // `creamui` cdylib's documented, stable C ABI (`creamui-ffi`), with
        // a function-pointer type matching that ABI's declared signature
        // exactly — this is the single place that contract is trusted.
        unsafe {
            Ok(Symbols {
                block_new: resolve!(lib, "creamui_block_new"),
                block_new_styled: resolve!(lib, "creamui_block_new_styled"),
                block_set_background: resolve!(lib, "creamui_block_set_background"),
                block_set_corner_radius: resolve!(lib, "creamui_block_set_corner_radius"),
                block_add_child: resolve!(lib, "creamui_block_add_child"),
                theme_dark: resolve!(lib, "creamui_theme_dark"),
                theme_light: resolve!(lib, "creamui_theme_light"),
                text_new: resolve!(lib, "creamui_text_new"),
                themed_text_new: resolve!(lib, "creamui_themed_text_new"),
                themed_text_secondary_new: resolve!(lib, "creamui_themed_text_secondary_new"),
                themed_text_new_sized: resolve!(lib, "creamui_themed_text_new_sized"),
                button_new: resolve!(lib, "creamui_button_new"),
                checkbox_new: resolve!(lib, "creamui_checkbox_new"),
                text_input_new: resolve!(lib, "creamui_text_input_new"),
                text_input_set_placeholder: resolve!(lib, "creamui_text_input_set_placeholder"),
                text_area_new: resolve!(lib, "creamui_text_area_new"),
                text_area_set_placeholder: resolve!(lib, "creamui_text_area_set_placeholder"),
                text_area_set_selection_colors: resolve!(
                    lib,
                    "creamui_text_area_set_selection_colors"
                ),
                slider_new: resolve!(lib, "creamui_slider_new"),
                scroll_view_new: resolve!(lib, "creamui_scroll_view_new"),
                widget_free: resolve!(lib, "creamui_widget_free"),
                signal_i32_new: resolve!(lib, "creamui_signal_i32_new"),
                signal_i32_get: resolve!(lib, "creamui_signal_i32_get"),
                signal_i32_set: resolve!(lib, "creamui_signal_i32_set"),
                signal_i32_free: resolve!(lib, "creamui_signal_i32_free"),
                signal_f32_new: resolve!(lib, "creamui_signal_f32_new"),
                signal_f32_get: resolve!(lib, "creamui_signal_f32_get"),
                signal_f32_set: resolve!(lib, "creamui_signal_f32_set"),
                signal_f32_free: resolve!(lib, "creamui_signal_f32_free"),
                signal_string_new: resolve!(lib, "creamui_signal_string_new"),
                signal_string_get: resolve!(lib, "creamui_signal_string_get"),
                signal_string_set: resolve!(lib, "creamui_signal_string_set"),
                signal_string_free: resolve!(lib, "creamui_signal_string_free"),
                window_resize: resolve!(lib, "creamui_window_resize"),
                window_set_position: resolve!(lib, "creamui_window_set_position"),
                window_set_always_on_top: resolve!(lib, "creamui_window_set_always_on_top"),
                window_handle_free: resolve!(lib, "creamui_window_handle_free"),
                app_builder_new: resolve!(lib, "creamui_app_builder_new"),
                app_builder_add_window: resolve!(lib, "creamui_app_builder_add_window"),
                app_builder_run: resolve!(lib, "creamui_app_builder_run"),
            })
        }
    }
}

/// A loaded `creamui` `cdylib` with every symbol this crate uses already
/// resolved. Cheap to pass around as `&Rc<Runtime>` — create one and share
/// it across every window/signal the app creates.
pub struct Runtime {
    // Never read after construction — kept only so the library stays mapped
    // for as long as any resolved function pointer in `sym` might be called.
    _lib: Library,
    pub(crate) sym: Symbols,
}

impl Runtime {
    /// Loads the `cdylib` at `path` and resolves every symbol this crate
    /// needs, eagerly, so a missing/mismatched symbol fails here rather
    /// than as an opaque crash the first time some unrelated widget is used.
    pub fn load(path: impl AsRef<Path>) -> Result<Rc<Runtime>, LoadError> {
        // SAFETY: loading an arbitrary shared library can run its
        // constructors, which is inherently unsound in general — the caller
        // is trusted to pass the path to a genuine `creamui` build.
        let lib = unsafe { Library::new(path.as_ref()) }.map_err(LoadError::Load)?;
        let sym = Symbols::load(&lib)?;
        Ok(Rc::new(Runtime { _lib: lib, sym }))
    }

    /// Convenience for examples/quick prototyping: locates the cdylib next
    /// to the current executable (matching where `cargo build` uplifts it),
    /// honoring `CREAMUI_LIB_PATH` as an override, and panics with a clear
    /// message on failure. Production apps that want to handle a missing
    /// library gracefully should call [`Runtime::load`] with their own path
    /// instead.
    pub fn load_default() -> Rc<Runtime> {
        let path = Self::locate_cdylib();
        Self::load(&path).unwrap_or_else(|e| {
            panic!(
                "creamui-dynamic: failed to load {path:?}: {e} (set CREAMUI_LIB_PATH to override)"
            )
        })
    }

    /// Locates the `creamui` cdylib next to the current executable, the way
    /// `cargo build` uplifts it (checking `deps/` too, since `cargo test`
    /// only builds it one level down there), or returns the
    /// `CREAMUI_LIB_PATH` override if set.
    pub fn locate_cdylib() -> PathBuf {
        if let Ok(override_path) = std::env::var("CREAMUI_LIB_PATH") {
            return override_path.into();
        }
        let mut dir = std::env::current_exe().expect("failed to resolve current executable path");
        dir.pop();
        let name = if cfg!(target_os = "macos") {
            "libcreamui.dylib"
        } else if cfg!(target_os = "windows") {
            "creamui.dll"
        } else {
            "libcreamui.so"
        };
        let sibling = dir.join(name);
        if sibling.exists() {
            sibling
        } else {
            dir.join("deps").join(name)
        }
    }

    /// Returns the bundled default dark theme's tokens.
    pub fn theme_dark(&self) -> CTheme {
        unsafe { (self.sym.theme_dark)() }
    }

    /// Returns the bundled default light theme's tokens.
    pub fn theme_light(&self) -> CTheme {
        unsafe { (self.sym.theme_light)() }
    }
}
