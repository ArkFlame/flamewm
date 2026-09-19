---
name: flamewm-performance-proof
description: Make FlameWM performance claims from repeatable measurements and retained evidence.
---

## Measurement contract

Define before optimization: user-visible workload, owning path, metric and unit, baseline revision, target threshold, sample count, environment, and exclusion criteria. Prefer latency distributions, throughput, frame pacing, allocation rate, CPU time, or resident memory over impressions.

## Method

1. Confirm correctness and instrumentation overhead before collecting a baseline.
2. Run equivalent, repeatable workloads with fixed relevant configuration and record raw output.
3. Attribute cost to canonical owner using profiler or tracing evidence; distinguish startup, steady-state, input, render, and teardown work.
4. Make one causal change at a time, preserving event ordering, WM authority, and cleanup semantics.
5. Re-run identical measurement. Compare distributions and variability, not only best run.

## Claims and retention

- Never call code shape, a single timing, or profiler intuition a performance improvement.
- Record machine, OS, display/runtime mode, command, revision, samples, raw evidence path, summary, and known noise.
- Reject gains that regress correctness, input responsiveness, memory bounds, or resource lifetime.
- State no conclusion when baseline and candidate are incomparable or noisy.
