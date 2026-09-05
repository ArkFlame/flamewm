# Agent Workspace

This directory defines the repository agent coordination contract.

- `PROTOCOL.md`: base operating rules and completion states.
- `CONTRACTS.md`: required packet, claim, lock, and job behavior.
- `SCHEMA.json`: machine-readable coordination packet schema.
- `JOBS.md`: job lifecycle and handoff rules.
- `runtime/`: local-only coordination state; never commit its contents.

Agents must read the applicable contract before editing. Work from repository-relative paths, keep changes bounded to the assigned job, preserve unrelated work, and report changed paths plus verification evidence. Do not claim build or test success when those checks were not run.

Runtime state uses `runtime/inbox/`, `runtime/outbox/`, `runtime/claims/`, and `runtime/locks/`. Empty runtime directories may be kept locally, but their files, status markers, and Python caches are ignored by Git.
