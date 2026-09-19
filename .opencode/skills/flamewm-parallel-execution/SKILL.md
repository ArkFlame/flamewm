---
name: flamewm-parallel-execution
description: Parallelize independent FlameWM investigation and edits without violating file ownership.
---

## Eligibility

Parallelize only work whose inputs, output paths, and verification do not depend on another active task. Parallel reads are preferred; serialize edits when ownership, API shape, generated artifacts, Cargo state, or runtime display state overlap.

## Protocol

1. Read handoff, `.agents` contract, Git state, and active claims.
2. Split work into explicit independent units with paths, owner, expected evidence, and stop condition.
3. Create one active claim per edit path. Never share a file between active editors.
4. Give each worker source context, constraints, and exact scope. Workers report evidence and blockers, not assumptions.
5. Re-read current files and Git diff before integrating results. Resolve source changes by preserving other owners' work; do not overwrite it.

## Guardrails

- One Cargo owner at a time. Do not run parallel Cargo builds, tests, formatting, or dependency operations.
- Serialize changes to contracts, manifests, generated files, global configuration, and shared architectural boundaries.
- Do not use concurrency to bypass a required source-first audit or native runtime isolation.
- Close claims and report task ownership, paths, commands, results, and unrun verification.

## Failure and return discipline

- Repeat failure in a lane: stop that lane regression-first (discriminating test, same-gate rerun) before a second tweak.
- Workers return compact capsules (<=80 lines); full evidence stays under `.agents/runtime/<cohort>/<job>/`.
- Integrator consumes only frozen contract capsules; re-read source and diff, never replay full worker logs.
