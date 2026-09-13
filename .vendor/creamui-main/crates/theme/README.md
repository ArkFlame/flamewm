# creamui-theme

Design tokens and theme state for CreamUI.

`Theme::default()` provides the rounded default dark appearance. `Theme::light()` keeps the same geometry and typography with a light palette. Applications can copy a theme and adjust individual color, spacing, type, and radius tokens.

```rust
use creamui_theme::{Color, Theme};

let mut theme = Theme::light();
theme.colors.accent = Color::rgb(72, 117, 255);
```

Part of [CreamUI](https://github.com/sammwyy/creamui), licensed under Apache-2.0.
