---
name: flamewm-runtime-evidence
description: Require direct runtime evidence for FlameWM native behavior and integration claims.
---

## Runtime proof standard

Native claims need fresh direct observation under a controlled environment. Static source, compilation, unit tests, screenshots without provenance, and prior logs are insufficient.

## Procedure

1. Define behavior, expected observation, owning process, inputs, timeout, cleanup, and isolation requirements.
2. Use repository-native build and launch paths. Prefer nested X server or isolated display for WM integration; never disrupt active desktop session.
3. Capture exact command, environment variables, binary/artifact identity, exit status, relevant protocol/process observations, and cleanup result.
4. Exercise success, cancellation/teardown, and fault path when claim depends on them.
5. Preserve bounded evidence under task-scoped ignored runtime area and summarize result without overstating coverage.

## Guardrails

- No runtime claim without verification: command was not run against current artifact, no claim.
- Keep display, sockets, processes, and temporary state isolated; terminate only processes started by task.
- Route authority, ICCCM/EWMH, grab, and coordinate observations to `flamewm-x11-window-manager`.
- State environmental limits, skipped paths, and remaining risk exactly.

## Rerun and return discipline

- Rerun the same gate that produced the failure before claiming recovery; a different passing gate is not proof.
- Prove failure mutation: same command, new observable signature tied to the fix.
- Return a compact capsule (<=80 lines): behavior, command/result, artifact identity, cleanup, limits; raw logs stay under `.agents/runtime/<cohort>/<job>/`.
