# RustWebRender 0.0.4 Validation Record

## Performed in artifact environment

- Inspected/preserved the supplied FlameWM HTML/CSS/JS source and target screenshot.
- Corrected the target geometry to the actual supplied image dimensions: `1350x641`.
- Audited adapted CSS declarations against the strict compiler property set.
- Verified every demo `<img>` reference exists.
- Verified all generated PPM assets are P6/255 with exact RGB payload sizing.
- Verified `pkg-config x11` reports 1.8.12 in the artifact environment.
- Previously verified the Xlib FFI prefix/field offsets used for XImage and XEvent against the installed 64-bit Xlib headers.
- Verified all required Xft dynamic symbols exist in `libXft.so.2`: `XftDrawCreate`, `XftDrawDestroy`, `XftDrawStringUtf8`, `XftFontOpenName`, `XftFontClose`, `XftColorAllocValue`, `XftColorFree`.
- Verified all required Fontconfig symbols exist in `libfontconfig.so.1`: `FcConfigGetCurrent`, `FcConfigAppFontAddFile`, `FcConfigBuildFonts`.
- Exercised an independent C ABI probe under Xvfb using the same signatures: process-local IBM Plex Sans registration, Xft font open, Xft color allocation and UTF-8 drawing all completed successfully.
- Exercised `scripts/prepare-ibm-plex` against the supplied IBM Plex Sans archive and verified all four expected faces are extracted and readable; `fc-scan` identifies the Regular face as `IBM Plex Sans`.
- Verified shell scripts with `bash -n`.
- Verified distribution policy scan excludes font binaries outside ignored `target/`.

## Not performed in artifact environment

The sandbox does not provide `rustc`, `cargo`, or Xephyr, and outbound toolchain installation is unavailable. Therefore no claim is made that `cargo test` or the actual Rust renderer was executed here.

Authoritative target-machine gates:

```bash
./doctor
./check
./xephyr
./verify-visual
```

`./check` runs workspace tests plus strict compile/decode of the full FlameWM example. `./verify-visual` renders the default state in a fresh 1350x641 Xephyr and reports native-size pixel difference against the supplied reference.

## 0.0.4 launcher repair validation

Performed in the artifact environment without claiming a real Rust build:

- `bash -n` over every shipped shell script.
- Executed top-level `./check` through its symlink with a bounded fake `cargo` that hard-failed unless `Cargo.toml` existed in the current working directory. The command resolved to the extracted 0.0.4 root and completed its compile/inspect control flow.
- Executed top-level `./demo` through its symlink with the same current-directory invariant.
- Executed top-level `./xephyr` through its symlink with bounded fake Xephyr/xdpyinfo/cargo commands; package-relative `scripts/prepare-ibm-plex` lookup no longer escaped to the parent directory.
- Executed `./verify-visual` with a tool layout containing `magick` but no standalone `import`; the harness selected `magick import`, completed capture/comparison control flow, and reported the expected synthetic zero-diff result.

These are launcher/control-flow tests only. They do not substitute for the real target-machine `cargo test`, native Xephyr render, or pixel comparison.

## 0.0.4 interactive Xephyr lifecycle repair

The packaging environment lacks Xephyr/Cargo, so the full target command could not be executed here. The root Xserver behavior was nevertheless executed against Xvfb, which accepts the same `-terminate`/`-noreset` lifecycle options:

```text
terminate ready=1 alive_after_probe=no
noreset   ready=1 alive_after_probe=yes
```

This validates the exact causal defect in 0.0.3: a short-lived readiness client can terminate a server started with `-terminate`, while `-noreset` keeps it alive. The 0.0.4 launcher uses `-noreset`, never `-terminate`, builds before starting Xephyr, verifies its spawned PID alongside display readiness, and runs the Rust client in the foreground with the selected nested `DISPLAY`.
