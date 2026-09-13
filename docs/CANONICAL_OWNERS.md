# Canonical Owners

| Mechanism | Owner | Boundary |
|---|---|---|
| Managed X11 windows and WM protocol | `crates/flamewm-wm` | typed WM commands/events |
| Native event sources | `crates/flamewm-reactor` | typed reactor events |
| D-Bus event source | `crates/flamewm-dbus-reactor` | typed D-Bus events |
| Feature/domain state | corresponding `*-core` crate | feature facade and typed contract |
| Platform policy | `crates/flamewm-platform` | platform facade |
| Desktop semantics | `crates/flamewm-desktop-core` | desktop facade |
| Settings semantics | `crates/flamewm-settings-core` | settings facade |
| Control protocol | `crates/flamewm-control-*` | control contract |
| Shell/UI projection | `crates/flamewm-shell`, `crates/flamewm-shell-core` | typed state and commands |
| UI renderer bridge | `crates/flamewm-ui-x11` | typed UI/backend contract |
| WM X11 bridge | `crates/flamewm-wm-x11` | typed WM/native contract |
| Flame X11 frame engine (window geometry, pointer hit, chrome scene) | `crates/flamewm-wm-x11/src/frame/**` + `decoration/interaction.rs` + `decoration/manager.rs` + `client.rs` | pure planners/intents/effects; live X applies |
| Render compilation | `crates/flamewm-render-compiler` | compiled render document |
| Layout, paint, hit testing | `crates/flamewm-render-core` | render commands |
| Native render output | `crates/flamewm-render-x11` | renderer-native boundary |
| Static-label CPU/span/counter/memory profiler | `crates/flamewm-profiler` | profiler facade (`start`, points, gauges, `report_window`) |

If a mechanism is not listed, add or clarify its owner before implementing a
second path. This index is an ownership guide, not a claim that every owner is
fully migrated or production-complete.

Architecture guard scans product facades, feature, bridge, and reactor crates,
including active `build.rs` scripts. It intentionally excludes canonical
renderer and native bridge owners listed above; those crates are the approved
boundary for renderer and X11 capabilities.

The guard also ratchets non-owner source shape by responsibility, rejects raw
FFI in product crates, and rejects blanket warning suppression. Native owners
remain responsible for documenting any unsafe function with its `# Safety`
contract; product unsafe functions are checked directly by the guard.
