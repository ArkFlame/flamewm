# FlameWM Identity-Preserving Porting Method

This rule is derived from the supplied IceWM Rust port post-mortem and is mandatory where **parity** is the objective.

## Core law

```text
COPY EXACT OWNER
-> TRANSLATE / RENAME MECHANICALLY
-> WIRE MINIMUM DEPENDENCY CLOSURE
-> COMPILE
-> DIFFERENTIAL TEST
-> FIX AT ORIGINAL OWNER
-> REFACTOR ONLY AFTER PARITY
```

Do not redesign a parity surface first and spend later releases repairing divergence.

## How this applies to FlameWM

### Browser V8 -> native WebRender

Parity matters. Preserve:

- exact presentation structure;
- metrics/geometry;
- source resource mappings;
- interaction ordering;
- defaults;
- state transitions;
- visible popup exclusivity.

JavaScript is not copied as a runtime dependency; each exact browser behavior is translated into native Rust controller state.

### Promoted renderer -> FlameWM Render

Parity/performance matters. Promote exact 0.0.6 owner files first, rename crate boundaries mechanically, then add FlameWM-specific typed mutations/integration.

### Historical IceWM/FlameWM implementations -> new FlameWM WM

This is **not** a parity fork anymore. FlameWM owns a new architecture. Historical source is used as a behavioral/edge-case oracle only. When copying a specific mature feature from reference source, copy its complete behavioral closure into project evidence/staging and translate semantics rather than reimplementing from a one-line description.

## Release law

```text
SOURCE_DRAFT -> COMPILES -> TESTED -> RUNTIME_VERIFIED -> DIFFERENTIAL_VERIFIED -> RELEASE
```

A stage may only be claimed when its gate actually ran. Static audit is not a Rust compiler surrogate.
