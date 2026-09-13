# Agent Protocol

Agents operate on repository-relative paths and preserve unrelated work. Read current source and Git state before editing. Keep each job bounded to its handoff, explicitly load all `REQUIRED_SKILLS`, and report changed paths plus verification evidence.

## Runtime Areas

- `runtime/inbox/`: incoming coordination packets
- `runtime/outbox/`: completed coordination packets
- `runtime/claims/`: active ownership claims
- `runtime/locks/`: short-lived coordination locks

Runtime files are local-only. Do not commit them or create status marker files.

## Evidence

Route evidence by job under `runtime/`; include job ID, role, loaded skills, commands, results, changed paths, blockers, and `runtime/hypotheses/<job-id>.md` when used. Ignored evidence is task-scoped; tracked contracts remain source of shared rules.

## Completion

Builder first reports `PATCH_APPLIED_NEEDS_VERIFY`, `BLOCKED`, or `FAILED`; mutating `PASS` is invalid. Verifier reports `PASS`, `BLOCKED`, or `FAILED` with exact commands and results. Coordinator reports `DONE` only after Verifier `PASS`; otherwise report `UNVERIFIED` or `BLOCKED` with route evidence. Never claim build or runtime success from inspection alone.
