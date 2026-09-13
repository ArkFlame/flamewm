# TODO

- `shell.toml` is only read at startup; there is no settings UI or live
  reload yet, so changes require restarting CreamShell.
- `[bar] position = "top"` only anchors correctly on the Wayland layer-shell
  backend (`creamui-platform`'s `WindowRole::TopPanel`/`BottomPanel`
  handling). The Windows backend does not place panel-role windows at all
  yet (pre-existing gap, not specific to the top/bottom option), so the bar
  position setting has no visible effect there.
