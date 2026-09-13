# creamui-fonts

Font registry, loading, and the `use_font()` hook for CreamUI.

CreamUI ships one bundled font (DejaVu Sans, family `"sans-serif"`) so text renders without depending on what's installed on the target machine. Register additional fonts and resolve a CSS-style family stack against them:

```rust
use creamui_fonts::{include_font, register_bytes, use_font, FontWeight};

register_bytes("Inter", FontWeight::Regular, include_font!("./Inter-Regular.ttf").to_vec()).unwrap();
register_bytes("Inter", FontWeight::Bold, include_font!("./Inter-Bold.ttf").to_vec()).unwrap();

// Inside a window's build_ui — see creamui_reactive::with_context_scope:
let font = use_font("Inter, sans-serif");
```

`register_file` loads from disk at runtime instead. `resolve` (family, weight) -> face is what `use_font` calls under the hood, for non-hook call sites.

Part of [CreamUI](https://github.com/sammwyy/creamui), licensed under Apache-2.0.
