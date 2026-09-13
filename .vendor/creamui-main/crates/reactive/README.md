# creamui-reactive

Small single-threaded reactive primitives for CreamUI.

Use `Signal<T>` to hold application state and `create_effect` to react to reads made during an effect. It is also useful as a lightweight dependency for small desktop tools that do not need the full CreamUI runtime.

```rust
use creamui_reactive::Signal;

let count = Signal::new(0);
count.update(|value| *value += 1);
assert_eq!(count.get(), 1);
```

Part of [CreamUI](https://github.com/sammwyy/creamui), licensed under Apache-2.0.
