# Images

`creamui-image` decodes an asset once into `ImageData` and reuses it across widget-tree rebuilds.

```toml
creamui-image = { version = "0.1", features = ["jpeg", "webp"] }
```

```rust
use creamui_image::{Image, ImageData, ImageFit};

let avatar = ImageData::from_bytes(include_bytes!("avatar.webp"))?;
let view = Image::new(avatar)
    .fit(ImageFit::Cover)
    .corner_radius(48.0);
```

Supported fits are `Fill`, `Contain`, `Cover`, and `None`. Rounded corners are clipped by the renderer, so square images can become avatars or circular thumbnails without preprocessing the file.

Run `cargo run -p images` in this repository for a PNG, JPEG, and WebP gallery.
