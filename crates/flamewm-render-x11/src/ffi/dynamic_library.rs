use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::{c_char, c_int, c_void};

const RTLD_NOW: c_int = 2;

pub(crate) struct DynamicLibrary {
    handle: *mut c_void,
}

impl DynamicLibrary {
    pub(crate) fn open(names: &[&str]) -> Result<Self, String> {
        for name in names {
            let c_name = CString::new(*name).expect("library name is static and NUL-free");
            // `c_name` remains alive for the call; `RTLD_NOW` is a valid dlopen flag.
            let handle = unsafe { dlopen(c_name.as_ptr(), RTLD_NOW) };
            if !handle.is_null() {
                return Ok(Self { handle });
            }
        }
        Err(format!("unable to load any of: {}", names.join(", ")))
    }

    pub(crate) unsafe fn symbol<T: Copy>(&self, name: &'static [u8]) -> Result<T, String> {
        debug_assert_eq!(name.last().copied(), Some(0));
        // SAFETY: `dlerror` takes no arguments; it only clears thread-local dl state.
        unsafe { dlerror() };
        // SAFETY: `self.handle` is a live `dlopen` handle owned until drop, and the
        // caller guarantees `name` is NUL-terminated per this function's contract.
        let raw = unsafe { dlsym(self.handle, name.as_ptr() as *const c_char) };
        // SAFETY: `dlerror` takes no arguments; it only reads thread-local dl state.
        let error = unsafe { dlerror() };
        if raw.is_null() || !error.is_null() {
            let message = if error.is_null() {
                "symbol resolved to NULL".to_string()
            } else {
                // SAFETY: `error` is non-NULL here; libdl guarantees a NUL-terminated
                // thread-local message valid until the next dl call on this thread.
                unsafe { CStr::from_ptr(error) }
                    .to_string_lossy()
                    .into_owned()
            };
            return Err(format!(
                "{}: {message}",
                String::from_utf8_lossy(&name[..name.len() - 1])
            ));
        }
        if mem::size_of::<T>() != mem::size_of::<*mut c_void>() {
            return Err("dynamic function pointer has unexpected size".to_string());
        }
        // SAFETY: `raw` is a non-NULL initialized data pointer and `T` was just
        // checked to have exactly pointer size, so a bitwise copy is sound.
        Ok(unsafe { mem::transmute_copy(&raw) })
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // Handle was returned by `dlopen` and remains owned until this drop.
            unsafe {
                dlclose(self.handle);
            }
        }
    }
}

#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}
