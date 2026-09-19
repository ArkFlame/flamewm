---
name: flamewm-semantic-modularization
description: Keep FlameWM modules aligned with one semantic owner per mechanism and typed feature boundaries.
---

## Ownership map

Read `docs/ARCHITECTURE.md`, `docs/ARCHITECTURE_EXCEPTIONS.toml`, current owner module, and callers before moving or adding behavior. Each mechanism has one canonical semantic owner.

- Product code calls typed facades; it does not access native sources directly.
- `flamewm-reactor` owns native event acquisition and typed dispatch.
- WM crates own authoritative window-management state and decisions.
- WebRender and UI consume typed projection; they never become WM authority.
- Exceptions are narrow explicit path entries only. No wildcard exception or accidental parallel owner.

## Change rules

1. Extend existing owner when capability already belongs there.
2. If boundary is unresolved, stop and route planning rather than creating a duplicate state machine, renderer, registry, or adapter.
3. Define typed input/output contract, authority, lifecycle, error behavior, and callers before introducing a new boundary.
4. Keep native access at designated boundary and propagate semantic events, not handles or implementation details.
5. Update all affected call sites as one coherent change; delete obsolete competing path only when scope explicitly includes it.

Report owner evidence, affected boundaries, exception entries if any, and validation limits.
