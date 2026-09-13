# creamui-image

Raster image support for CreamUI.

PNG decoding is enabled by default. Enable only the extra codecs your application needs:

```toml
creamui-image = { version = "0.1", features = ["jpeg", "webp"] }
```

Decode bytes into reusable `ImageData`, then choose a fit and optional rounded crop with `Image`.

```rust
let image = Image::new(ImageData::from_bytes(bytes)?)
    .fit(ImageFit::Cover)
    .corner_radius(24.0);
```

See the [image guide](https://github.com/sammwyy/creamui/blob/main/docs/images.md) for details.
