---
name: flamewm-failure-resolution
description: Resolve FlameWM failures from exact evidence with bounded, root-cause repairs.
---

## Evidence-first triage

1. Capture exact error, command, exit status, environment, version, and earliest relevant log or backtrace.
2. Reproduce only with repository-native commands and bounded scope. Do not erase diagnostic artifacts.
3. Trace from failure boundary to canonical owner, including inputs, state transitions, error propagation, cleanup, and caller expectations.
4. Form competing root-cause hypotheses when evidence is ambiguous. Record discriminating evidence in task-scoped hypothesis ledger.

## Repair discipline

- Repair owning mechanism, not downstream symptoms, retries, broad catches, or silent fallbacks.
- Keep patch minimal; update all impacted call sites and contracts atomically.
- Preserve error identity and useful context. Do not log and rethrow the same error unless existing contract requires it.
- After one failed repair, compare hypotheses against diagnostics. Stop after bounded attempts and report exact blocker rather than guessing.
- Route Rust, X11, runtime, or performance defects to their matching FlameWM skills.

## Completion evidence

Report reproduction command and result, root-cause evidence, changed paths, verification command and result, unrun checks, and remaining risk. Static inspection cannot establish native runtime recovery.

## Regression-first discipline

- Repeat bug: add or name the discriminating regression test first; rerun the same gate that failed before any new tweak.
- Failure mutation: prove the fix changes the recorded failure signature (command, exit, log line), not just compiles.
- No second tweak without a new discriminating test or observation separating the remaining hypotheses.
- Return a compact capsule (<=80 lines): status, paths, commands/results, blocker; full logs stay under `.agents/runtime/<cohort>/<job>/`.
