# Overnight execution capsule — run 20260909T023239Z

- Baseline HEAD: `ecda842cac139b37850aeb1cd86d18df1c9e3fb7`
- Branch: `main`
- Baseline worktree: clean
- Host display: `:0` (KWin; no Flame PID, no TERM action)
- Usable baseline run: `.performance/20260909T013140Z-1339532`
- Wave 1 GREEN: J01 profiler p95/rank fix, J02 Xephyr env guard,
  J03 Control bootstrap/revisions, J04 parallel icon service,
  J05 renderer image cache bound/telemetry, J06 transactional geometry.
- Wave 2 GREEN (source + compile + test): J07 shell bootstrap/events,
  J08 Start icon consumer, J09 desktop icons/sticky, J10 popup lifecycle,
  J11 decoration painter/cursor/toggle, J12 CSD policy (ClientDecorated
  reserved; no verified GTK CSD atom in source/reference).
- Verification: static audit OK, architecture guard OK (2 ratified
  exceptions), `cargo fmt` OK, `cargo test --workspace` all GREEN,
  `cargo check --workspace --all-targets` OK, release build OK for
  flamewm-wm/flamewm-desktop/flamewm-shell. Clippy unavailable
  (`no such command: clippy` => ENVIRONMENT-BLOCKED per handoff).
- Xephyr smoke: FLAMEWM_XEPHYR_CANARY_PASS display=:97 version=0.0.8,
  run `.performance/20260909T034528Z-1448610`. No p95 sentinel values;
  shell.startup.total=2878ms (project_panel=0.07ms, bootstrap path live).
  PSS capture shows 360kB entries (early-capture artifact, needs warm read).
- New source files (untracked, intentional): icon_service.rs,
  shell_bootstrap.rs, decoration/policy.rs, capsule dir.
- READY frontier: Wave 3 P0 gauntlet (G01-G04), Wave 4 interaction (I01-I06),
  Wave 5 measured performance loop.
