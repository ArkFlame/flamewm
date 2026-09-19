---
name: flamewm-rust-systems
description: Implement Rust systems changes with explicit ownership, events, lifetimes, and native boundaries.
---

## Ownership and state

Model ownership, mutation authority, thread/loop affinity, event order, cancellation, and teardown in types and control flow. Prefer explicit state machines and typed messages over shared mutable globals, callback cycles, or lifetime extension.

## Async and events

- Identify producer, consumer, ordering guarantee, backpressure behavior, cancellation owner, and terminal state for each event path.
- Do not hold locks across blocking, callback, await-like, X11, or renderer boundaries.
- Fence stale generations after reconnect, restart, replacement, or teardown; late events must not mutate newer state.
- Make shutdown idempotent and ensure resources release on success, error, cancellation, and drop paths.

## Native boundary

- Keep unsafe code out of workspace-inherited lint domains. Confine unavoidable FFI to designated boundary crate and smallest audited block.
- Document pointer ownership, mutability, validity interval, callback threading, destruction function, and error convention at FFI boundary.
- Never expose native handles or untyped event sources as product authority; convert them to typed contracts through canonical owner.

## Change proof

Inspect callers and error/teardown paths. Compile and run scoped checks only when handoff permits. This rust change proof routes X11 behavior and release/runtime claims to matching skills.
