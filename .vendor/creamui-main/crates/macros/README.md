# creamui-macros

JSX syntax and typed component helpers for CreamUI.

```rust
use creamui_macros::jsx;
use creamui_core::layout::FlexDirection;
use creamui_widgets::layout::{Align, Justify};

let screen = jsx! {
    <Flex direction={FlexDirection::Column} gap={12.0}
        align={Align::Center} justify={Justify::Center}>
        <Button on_click={|| save()}>"Save"</Button>
    </Flex>
};
```

The macro expands to ordinary Rust constructors, so component names, imports, props, and callback types are checked by the compiler.
