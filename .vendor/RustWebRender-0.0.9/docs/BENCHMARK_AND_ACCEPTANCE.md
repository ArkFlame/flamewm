# RustWebRender Performance and Acceptance Gates

No performance claim should be accepted from architecture alone.

## Metrics

Measure at minimum:

- PSS of the FlameWM process/runtime owner;
- RSS only as secondary context;
- binary text/data size delta;
- cold start wall time;
- first panel paint latency;
- relayout latency after representative task-count changes;
- repaint latency for hover/focus changes;
- idle CPU percentage;
- idle wakeups per second;
- allocations during steady-state repaint if profiling tools permit.

## Baseline comparison

Compare the same visual shell implemented through:

1. current hand-coded Rust/native renderer;
2. RustWebRender core + native canvas adapter;
3. optional future Taffy-backed layout;
4. only if justified, any broader engine such as Blitz.

Do not compare a full browser-demo process against an in-process panel and conclude anything about the architecture. The comparison must preserve process boundaries and functionality.

## Proposed gates for FlameWM adoption

RustWebRender is acceptable only if all are true:

- idle PSS increase is small enough to preserve FlameWM's approximately 40 MiB owned-process target;
- no persistent renderer thread is required;
- no periodic redraw loop is required;
- startup regression is operationally negligible;
- taskbar hover/action latency is visually immediate;
- generated UI can reproduce the required FlameWM shell without unsupported-property workarounds spreading through CSS;
- native service ownership stays outside the UI document.

## Included benchmark helper

```bash
./scripts/bench-memory
```

The script builds release binaries, launches the example in Xephyr and reports PSS/RSS through `smem`.

It intentionally does not contain hardcoded pass/fail numbers because the meaningful gate is the delta inside the target FlameWM build on the target machine.

## Validation status of this archive

The source archive was assembled in an environment that did not contain `rustc` or `cargo`, so a real Rust compile/test gate could not be executed there. The project includes unit tests and `./check` specifically so the first environment with a Rust toolchain surfaces any compiler/API mistake immediately rather than hiding it behind claimed results.
