# Agent Contracts

## Scope

An agent owns only the files and behavior named by its handoff. It must inspect current source and Git state first, avoid unrelated cleanup, and leave the repository compilable after each coherent change when verification is in scope.

## Coordination Packets

Packets are UTF-8 JSON files validated by `SCHEMA.json`.

- Inbox packets are requests received by an agent.
- Outbox packets are completion reports sent to the parent or coordinator.
- Packet filenames are stable, descriptive, and use `.json`.
- A packet identifies one job, its owner, scope, requested outcome, and verification evidence.
- Missing, ambiguous, or conflicting requirements are reported as `BLOCKED`; agents do not silently invent contract changes.

## Claims and Locks

- A claim records the job ID, agent, paths, start time, and state.
- One active claim owns a path at a time.
- A lock is short-lived and protects coordination metadata only; it is not a substitute for a claim.
- Claims and locks are local runtime state and must not be committed.
- Remove or close coordination state when the job completes or is abandoned.

## Completion

Every completion packet uses exactly one state:

- `DONE`: requested work completed and required verification passed.
- `UNVERIFIED`: requested work completed, but an allowed verification step was not run.
- `BLOCKED`: work could not be completed; include the exact blocker and smallest next action.

Reports list changed paths, verification commands and results, remaining risks, and blockers. Never report success from inspection alone.
