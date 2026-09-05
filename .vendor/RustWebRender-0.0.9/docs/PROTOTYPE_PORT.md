# FlameWM V8 -> RustWebRender 0.0.4 Prototype Port

## Source authority

The user-supplied browser prototype is preserved under `reference/flamewm-html-css-js-v8/`. Its `PORTING_CONTRACT.md` explicitly treats HTML/CSS/JavaScript as disposable specification code and native behavior as authoritative. For this renderer experiment, the same separation is used: HTML/CSS remains the presentation language, while JavaScript behavior is represented by Rust actions/controller state.

## Visual acceptance state

`reference/current-0.0.1.png` is the failed baseline: navy desktop, white taskbar/menu, bitmap-small appearance and no visible/defined cursor.

`reference/target-xephyr.png` is the target state: black desktop, FlameWM/Breeze dark shell, System Settings/About window at the prototype geometry, desktop context menu, bottom taskbar, Trash and watermark.

The 0.0.4 adaptation materializes that state in `examples/flamewm-v8/index.html` + `flamewm.css` rather than hard-coded Rust painting calls.

## JS -> native controller mapping

| Browser reference behavior | 0.0.4 native experiment |
| --- | --- |
| Start menu class/state toggles | `RuntimeDocument::set_visible("start-menu", ...)` |
| Desktop `contextmenu` | secondary X11 button -> `desktop.surface` `ActionEvent` -> geometry override + visibility |
| Settings sidebar re-render | seven precompiled page subtrees; Rust toggles visibility + selection geometry |
| Settings close/minimize | hide `settings-window` |
| Settings maximize | runtime position/size override |
| Taskbar Settings restore | show `settings-window` |
| Open Terminal | show precompiled `terminal-window` subtree |
| Pointer cursor | compiled CSS cursor -> native X cursor font glyph |
| Browser images/SVG | build-time PPM raster assets embedded in `RWRB/2` |

No JavaScript interpreter or generic DOM mutation layer was added.

## Intentionally not claimed in 0.0.4

Full V8 window manager simulation, task-entry drag/reorder, real application launch, real audio/network/media services, desktop icon persistence, sticky-note editor, hotkey recording, display reconfiguration, taskbar docking, workspace switching and full form control semantics are not ported. Those are broader prototype behaviors and would obscure the renderer experiment.

0.0.4 proves the high-value renderer boundary first: can the attached visual system be expressed in HTML/CSS and driven by small native Rust state changes while remaining a direct native X11 process?
