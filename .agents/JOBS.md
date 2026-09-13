# Jobs

## Lifecycle

1. Read `PROTOCOL.md`, this file, and the job packet. Explicitly load every `REQUIRED_SKILLS` named by the handoff.
2. Inspect the relevant source and Git state before editing.
3. Create one claim for the job and acquire coordination locks only when needed.
4. Make the smallest change that satisfies the handoff. Do not widen scope silently.
5. Run the project-native verification required by the handoff. If verification is out of scope, say so.
6. Write one completion packet to `runtime/outbox/` using `SCHEMA.json`, including route evidence and any `runtime/hypotheses/<job-id>.md` reference.
7. Close the claim and release locks.

## Handoffs

A handoff must name a unique job ID, owner, requested outcome, allowed paths, constraints, and verification requirements. The receiving agent owns interpretation only within that scope. If a handoff cannot be found or does not contain enough information, report `BLOCKED` rather than guessing.

## Job States

- `PENDING`: accepted but not started.
- `IN_PROGRESS`: claim is active and work is underway.
- `DONE`: implementation and required verification are complete.
- `UNVERIFIED`: implementation is complete but required verification could not run or was excluded.
- `BLOCKED`: implementation cannot proceed; the report must name the blocker and smallest next action.

## Role States

- Coordinator assigns required skills, Builder, Verifier, and task-scoped evidence route before mutation.
- Builder first reports `PATCH_APPLIED_NEEDS_VERIFY`, `BLOCKED`, or `FAILED`; `PASS` is invalid for a Builder mutation report.
- Verifier reports `PASS`, `BLOCKED`, or `FAILED` with independent commands and results. Only Verifier `PASS` permits Coordinator `DONE`.
- Ignored evidence is task-scoped under `runtime/` and names its job. Shared decisions must be written to tracked contracts instead.

## File Rules

All paths in packets are repository-relative. Runtime files stay under `.agents/runtime/`; never add runtime state, status markers, claims, locks, inbox packets, outbox packets, or `__pycache__` to a commit. Preserve existing user or agent changes, and never use destructive repository-wide reset or checkout operations.
