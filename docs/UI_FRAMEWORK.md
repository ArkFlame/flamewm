# UI Framework Boundary

FlameWM UI is a typed projection over product state. `flamewm-shell` translates
interaction into commands; feature cores and facades remain authoritative.

- UI may render only through the FlameWM renderer pipeline.
- UI must not call raw X11, D-Bus, filesystem, process, or device APIs.
- UI must consume typed snapshots/events and issue typed commands through the
  owning feature facade.
- UI must not maintain a parallel WM, workspace, settings, or integration model.
- Native event subscription and lifecycle belong to the reactor owner.
- A renderer change belongs in the corresponding render crate, not in a feature
  controller or UI asset as a hidden mechanism.

The framework boundary is architectural even where current implementation work
is incomplete; no document here claims migration completion.
