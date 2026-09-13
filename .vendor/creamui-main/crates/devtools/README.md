# creamui-devtools

Development-only tooling for CreamUI applications. Call `init()` before
opening windows; press F3 in a CreamUI window to show or hide its overlay.

Use it either directly (typically as a development dependency):

```toml
[dev-dependencies]
creamui-devtools = "0.1"
```

or enable `creamui`'s `devtools` feature and call `creamui::devtools::init()`.

```rust
fn main() {
    creamui_devtools::init();
    // creamui::run(...)
}
```
