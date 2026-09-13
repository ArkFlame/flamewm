# creamui-abi

Plain `#[repr(C)]` types shared by the CreamUI dynamic runtime.

This crate has no engine dependency. Most Rust applications do not need it directly; it exists for `creamui-ffi` producers and `creamui-dynamic` consumers that communicate through the optional C ABI.

Learn more in the [ABI guide](https://github.com/sammwyy/creamui/blob/main/docs/ffi.md).
