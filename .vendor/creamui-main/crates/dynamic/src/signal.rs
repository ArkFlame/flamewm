//! Reactive value handles — the dlopen-backed equivalent of
//! `creamui_reactive::Signal<T>`. Reading one (via `.get()`) while building
//! a widget tree inside a `build_ui` closure subscribes that window's next
//! render to future writes, exactly like the native `Signal`.

use crate::runtime::Runtime;
use std::ffi::{c_void, CStr, CString};
use std::rc::Rc;

struct RawSignalI32 {
    ptr: *mut c_void,
    rt: Rc<Runtime>,
}

impl Drop for RawSignalI32 {
    fn drop(&mut self) {
        unsafe { (self.rt.sym.signal_i32_free)(self.ptr) };
    }
}

/// A reactive `i32` value. Cheap to clone — every clone shares the same
/// underlying signal, freed once the last clone drops.
#[derive(Clone)]
pub struct SignalI32(Rc<RawSignalI32>);

impl SignalI32 {
    pub fn new(rt: &Rc<Runtime>, initial: i32) -> Self {
        let ptr = unsafe { (rt.sym.signal_i32_new)(initial) };
        SignalI32(Rc::new(RawSignalI32 {
            ptr,
            rt: rt.clone(),
        }))
    }

    pub fn get(&self) -> i32 {
        unsafe { (self.0.rt.sym.signal_i32_get)(self.0.ptr) }
    }

    pub fn set(&self, value: i32) {
        unsafe { (self.0.rt.sym.signal_i32_set)(self.0.ptr, value) };
    }
}

struct RawSignalF32 {
    ptr: *mut c_void,
    rt: Rc<Runtime>,
}

impl Drop for RawSignalF32 {
    fn drop(&mut self) {
        unsafe { (self.rt.sym.signal_f32_free)(self.ptr) };
    }
}

/// A reactive `f32` value. Same clone/free semantics as [`SignalI32`].
#[derive(Clone)]
pub struct SignalF32(Rc<RawSignalF32>);

impl SignalF32 {
    pub fn new(rt: &Rc<Runtime>, initial: f32) -> Self {
        let ptr = unsafe { (rt.sym.signal_f32_new)(initial) };
        SignalF32(Rc::new(RawSignalF32 {
            ptr,
            rt: rt.clone(),
        }))
    }

    pub fn get(&self) -> f32 {
        unsafe { (self.0.rt.sym.signal_f32_get)(self.0.ptr) }
    }

    pub fn set(&self, value: f32) {
        unsafe { (self.0.rt.sym.signal_f32_set)(self.0.ptr, value) };
    }
}

struct RawSignalString {
    ptr: *mut c_void,
    rt: Rc<Runtime>,
}

impl Drop for RawSignalString {
    fn drop(&mut self) {
        unsafe { (self.rt.sym.signal_string_free)(self.ptr) };
    }
}

/// A reactive `String` value. Same clone/free semantics as [`SignalI32`].
#[derive(Clone)]
pub struct SignalString(Rc<RawSignalString>);

impl SignalString {
    pub fn new(rt: &Rc<Runtime>, initial: &str) -> Self {
        let c_initial = CString::new(initial).unwrap_or_default();
        let ptr = unsafe { (rt.sym.signal_string_new)(c_initial.as_ptr()) };
        SignalString(Rc::new(RawSignalString {
            ptr,
            rt: rt.clone(),
        }))
    }

    /// Returns an owned copy of the current value — unlike the raw ABI
    /// entry point this wraps, there's no borrowed-pointer lifetime to
    /// manage.
    pub fn get(&self) -> String {
        let ptr = unsafe { (self.0.rt.sym.signal_string_get)(self.0.ptr) };
        if ptr.is_null() {
            return String::new();
        }
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }

    pub fn set(&self, value: &str) {
        let c_value = CString::new(value).unwrap_or_default();
        unsafe { (self.0.rt.sym.signal_string_set)(self.0.ptr, c_value.as_ptr()) };
    }
}
