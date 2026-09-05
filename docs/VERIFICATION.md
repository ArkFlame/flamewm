# Verification — 0.0.2

## Mandatory target-machine gate

```bash
./doctor
./check
./xephyr
```

`./check` executes:

1. static architecture/dependency audit;
2. FlameWM Render 0.0.6 source promotion audit;
3. Cargo metadata;
4. workspace/all-target compiler check;
5. workspace tests;
6. complete release workspace build;
7. V8 HTML/CSS -> RWRB/2 release compile;
8. RWRB/2 inspect smoke gate;
9. active-package font-binary guard.

Formatting/Clippy is intentionally a separate hygiene gate:

```bash
./lint
```

It must not obscure the compiler/runtime gate when diagnosing a release candidate.

## Visual gate

```bash
./verify-visual
```

The current and approved images must have identical geometry. The script reports RMSE and absolute-error metrics without resizing either image.

Reference:

```text
project_sources/visual/rwr-0.0.6-target-xephyr.png
```

## Packaging-environment limitation

The artifact assembly environment does not provide Rust/Cargo/Xephyr. Therefore this archive is source-audited and packaged, but no compiler/runtime PASS is invented. The target-machine commands above determine `COMPILES`, `TESTED`, `RUNTIME_VERIFIED` and `DIFFERENTIAL_VERIFIED` status.
