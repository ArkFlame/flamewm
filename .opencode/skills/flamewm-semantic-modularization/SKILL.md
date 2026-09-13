---
name: flamewm-semantic-modularization
description: Keep FlameWM modules aligned with one semantic owner per mechanism and typed feature boundaries.
---

Extend existing canonical owner; do not duplicate state machines, renderers, event sources, or native access. Product features use facades and typed contracts. `flamewm-reactor` owns native event sources and typed dispatch. WebRender owns presentation only; WM state remains authoritative in WM owners. Add architecture exceptions only as explicit path-level entries.
