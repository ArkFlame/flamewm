# Agent Protocol

Agents operate on repository-relative paths and preserve unrelated work. Read current source and Git state before editing. Keep each job bounded to its handoff, and report changed paths plus verification evidence.

## Runtime Areas

- `runtime/inbox/`: incoming coordination packets
- `runtime/outbox/`: completed coordination packets
- `runtime/claims/`: active ownership claims
- `runtime/locks/`: short-lived coordination locks

Runtime files are local-only. Do not commit them or create status marker files.

## Completion

Report `DONE`, `UNVERIFIED`, or `BLOCKED` with the exact verification command and result. Never claim build or runtime success from inspection alone.
