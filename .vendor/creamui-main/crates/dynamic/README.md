# creamui-dynamic

Safe Rust client for a dynamically loaded CreamUI runtime.

It loads the shared library built by `creamui-ffi` and offers owned widget handles plus an `AppBuilder` API. Use the native CreamUI crates for standard Rust applications; choose this crate when a plugin host or desktop shell needs one shared runtime.

See the [ABI guide](https://github.com/sammwyy/creamui/blob/main/docs/ffi.md).
