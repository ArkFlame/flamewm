---
name: flamewm-reference-mining
description: Mine historical FlameWM reference corpora as evidence without making them production dependencies.
---

## Scope

`.vendor/**` is historical/reference evidence only. Never edit it, add it as a build dependency, include it, link it, load it, execute it, copy generated output from it, or ship it as runtime input.

## Mining procedure

1. State question and active FlameWM owner before searching reference material.
2. Locate exact upstream symbol, version/context, callers, and related tests or documentation.
3. Extract semantic behavior: inputs, state transitions, ordering, ownership, failure handling, interoperability constraints, and observable outcomes.
4. Compare reference behavior against active source and current architecture. Reference does not override current contracts without explicit owner decision.
5. Reimplement only necessary behavior in active source using project types and boundaries; do not transplant code, layouts, or dependencies.

## Evidence record

Report reference paths, revision/context when known, semantic finding, active implementation path, and validation method. Mark uncertainty where corpus is incomplete. Route native X11 findings through `flamewm-x11-window-manager`.
