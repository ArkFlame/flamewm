# RWR 0.0.9 Skin Promotion Map

Source: `.vendor/RustWebRender-0.0.9/examples/flamewm-v8/flamewm.css` and `index.html`.
The crate is renderer-neutral; asset paths target existing product assets.

## Central Tokens

| Promoted group | Product path | Source path/rule |
| --- | --- | --- |
| Palette and alpha overlays | `crates/flamewm-skin/src/palette.rs` | `flamewm.css:1`, `:hover`/selection rules at 8, 17-18 |
| Typography | `crates/flamewm-skin/src/typography.rs` | `flamewm.css:3,10,25,45-46,71,121,222-224` |

## Metric Groups

| Group | Product path | Exact promoted values | Source path/rule |
| --- | --- | --- | --- |
| Desktop icon | `metrics.rs`, `recipes/desktop.rs` | 82x82; icon 44; label 76x15; gap 4; radius 4; padding 3x4 | `flamewm.css:7,9-10` |
| Window chrome | `metrics.rs`, `recipes/window_chrome.rs` | radius 6; titlebar 31; title sides/controls 114; button 38x31; icon 13 | `flamewm.css:19,22-27,32` |
| Settings | `metrics.rs`, `recipes/settings.rs` | window 720x480; nav 192; toggle 40x22; knob 18 | `flamewm.css:20,34,59-62` |
| Start menu | `metrics.rs`, `recipes/start.rs` | width 292; min-height 349; app row 36 | `flamewm.css:83,99` |
| Taskbar and pager | `metrics.rs`, `recipes/taskbar.rs` | taskbar 44; pager 51x44; task button 42; workspace button 22x16 | `flamewm.css:108,110,115,164-166` |
| Sticky note | `metrics.rs`, `recipes/sticky.rs` | 190x190; padding 12; gap 8 | `flamewm.css:221-224` |
| Context menu | `recipes/menu.rs` | width 205; row 195x31; separator 185; padding 5; radius 4 | `flamewm.css:102-106` |

## Semantic Icons

`crates/flamewm-skin/src/icons.rs` maps roles to existing `assets/web/breeze/*.svg`
or branding assets. It intentionally does not model state variants that are not
represented by an explicit role.
