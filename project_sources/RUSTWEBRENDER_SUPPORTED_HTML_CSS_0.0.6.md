# RustWebRender 0.0.4 HTML/CSS Profile

This is a strict system-UI language, not a browser compatibility promise.

## HTML

Supported: nested elements, text, attributes, comments/doctype, basic entities, `<style>`, local `<link rel="stylesheet">`, `id`, `class`, inline `style`, `data-action`, and local `<img src="...">`.

`<img>` is a leaf node. In 0.0.4 the build input must be binary PPM (`P6`, max value 255). The compiler embeds opaque RGB8 pixels into the compiled document. Network/data URLs are rejected.

`head`, `style`, `link`, `meta`, `title` and `script` are compile-time-only/not emitted. JavaScript is never executed.

## Selectors

Supported: `*`, tag, `.class`, `#id`, compounds, descendant, child `>`, comma lists, `:root`, `:hover`, `:active`.

Attribute selectors, sibling combinators, pseudo-elements and other pseudo-classes fail strict compilation.

## CSS

Supported declarations:

- `display: none | block | flex`
- `flex-direction: row | column`
- `justify-content: start | center | end | space-between` plus flex aliases
- `align-items: start | center | end | stretch` plus flex aliases
- `flex-grow`, single-number `flex`
- `position: static | relative | absolute`
- width/height/min/max/top/right/bottom/left
- px, %, auto and zero lengths
- margin/padding shorthand and individual sides
- gap
- background/background-color (color only)
- color
- border color/width/radius and `<px> solid <color>` shorthand
- font-size/font-weight metadata
- opacity metadata
- `cursor: default | auto | pointer | text | move | grab | grabbing | ew-resize | ns-resize | nwse-resize | nesw-resize` and directional aliases
- `box-sizing: border-box` (accepted/no-op because RWR sizing is already border-box-oriented)
- hidden/visible overflow declarations (accepted/no-op; no scrolling)
- basic `user-select` declarations (accepted/no-op)

## Colors

`transparent`, black/white/red/gray, `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, integer `rgb()`, numeric-alpha `rgba()` and typed color variables:

```css
:root { --accent: #ef4048; }
button { background: var(--accent); }
```

Custom properties remain color-only runtime slots.

## Deliberately unsupported

Grid, inline formatting/shaping, floats, calc/min/max/clamp, viewport/rem/em units, transforms, shadows, gradients, transitions, animations, scrolling, z-index/stacking contexts, generated content, arbitrary custom-property token streams, `!important`, runtime URL loading and JS.
