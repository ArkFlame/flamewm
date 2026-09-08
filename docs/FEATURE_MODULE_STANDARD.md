# Feature Module Standard

Each feature is a narrow vertical slice with explicit ownership:

1. `feature-core` contains pure state, domain types, and typed events/commands.
2. The feature facade owns policy and is the supported caller boundary.
3. Adapters connect platform, reactor, persistence, or control contracts.
4. A UI/shell projection displays state and translates user intent only.

Rules:

- Name the canonical owner before adding code.
- Keep native access in the platform, WM, renderer, or reactor owner.
- Keep I/O and source lifecycle out of pure core crates.
- Do not expose raw native handles to product/UI callers.
- Do not add a second state authority for convenience.
- Document incomplete work as incomplete; do not infer completion from wiring alone.

Handoffs must list changed paths, owner, facade/contract, verification command,
and any exact architecture exception.
