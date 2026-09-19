# Agent Workspace

This directory defines the repository agent coordination contract.

- `PROTOCOL.md`: base operating rules and completion states.
- `CONTRACTS.md`: required packet, claim, lock, and job behavior.
- `SCHEMA.json`: machine-readable coordination packet schema.
- `JOBS.md`: job lifecycle and handoff rules.
- `runtime/`: local-only coordination state; never commit its contents.

Agents must read the applicable contract before editing. Explicitly load every handoff `REQUIRED_SKILLS` before work; report loaded skills in route evidence. Record the route ledger as UTF-8 JSON under `runtime/`: `job`, `role`, `source_handoff`, `route`, `changed_paths`, `commands`, `blockers`, and `hypothesis_ledger`. `route` is the `skill_route.py` JSON object with `required`, `optional`, and `reasons`. Work from repository-relative paths, keep changes bounded to the assigned job, preserve unrelated work, and report changed paths plus verification evidence. Do not claim build or test success when those checks were not run.

Runtime state uses `runtime/inbox/`, `runtime/outbox/`, `runtime/claims/`, and `runtime/locks/`. Empty runtime directories may be kept locally, but their files, status markers, and Python caches are ignored by Git.

Task-scoped ignored evidence belongs under `runtime/` and must identify its job. Shared coordination rules belong in tracked contract files, never ignored evidence.
