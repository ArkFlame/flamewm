# Dynamic and C ABI usage

Most Rust applications should use the native crates directly. CreamUI also offers an optional ABI path for applications that need to load a shared runtime at process startup or from another language.

- `creamui-abi` contains the plain `#[repr(C)]` value types.
- `creamui-ffi` builds the `creamui` shared library and exports C functions.
- `creamui-dynamic` is the safe Rust client that loads that library with `libloading`.

The ABI is intentionally separate from the normal Rust widget API. It is useful for plugin hosts and shared desktop runtimes, but it is not required for ordinary native applications.

See the public Rust API documentation for `creamui-dynamic::AppBuilder` and the exported declarations in `creamui-ffi` when integrating this path.
