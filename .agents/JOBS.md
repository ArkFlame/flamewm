# Jobs

## Lifecycle

1. Read `PROTOCOL.md`, this file, and the job packet.
2. Inspect the relevant source and Git state before editing.
3. Create one claim for the job and acquire coordination locks only when needed.
4. Make the smallest change that satisfies the handoff. Do not widen scope silently.
5. Run the project-native verification required by the handoff. If verification is out of scope, say so.
6. Write one completion packet to `runtime/outbox/` using `SCHEMA.json`.
7. Close the claim and release locks.

## Handoffs

A handoff must name a unique job ID, owner, requested outcome, allowed paths, constraints, and verification requirements. The receiving agent owns interpretation only within that scope. If a handoff cannot be found or does not contain enough information, report `BLOCKED` rather than guessing.

## Job States

- `PENDING`: accepted but not started.
- `IN_PROGRESS`: claim is active and work is underway.
- `DONE`: implementation and required verification are complete.
- `UNVERIFIED`: implementation is complete but required verification could not run or was excluded.
- `BLOCKED`: implementation cannot proceed; the report must name the blocker and smallest next action.

## File Rules

All paths in packets are repository-relative. Runtime files stay under `.agents/runtime/`; never add runtime state, status markers, claims, locks, inbox packets, outbox packets, or `__pycache__` to a commit. Preserve existing user or agent changes, and never use destructive repository-wide reset or checkout operations.
