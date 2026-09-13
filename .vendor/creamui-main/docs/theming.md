# Theming

`Theme::default()` is CreamUI's single default visual language: rounded surfaces, filled selection, balanced spacing, and a dark color scheme.

`Theme::light()` keeps the same component geometry and typography while using the light palette. `Theme::dark()` is equivalent to the default dark palette.

```rust
let theme = Theme::default();
let light_theme = Theme::light();
```

## Customize tokens

`Theme` is a regular copyable value. Start from the default and adjust the tokens that matter to your application.

```rust
let mut theme = Theme::light();
theme.colors.accent = Color::rgb(72, 117, 255);
theme.card_radius = 16.0;
theme.spacing_large = 20.0;
```

Themed widgets read their colors, typography, spacing, and component radii from the value you pass them. Keep a single theme in application state when you want a live appearance switcher.

## Use raw widgets for exceptions

Use `Raw*` widgets for a component that intentionally does not follow your main theme. This keeps special visual treatments explicit rather than adding one-off rules to every themed component.
