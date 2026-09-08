# FlameWM Agent Governance

This repository is FlameWM 0.0.7. Repository-local agent rules are defined in
`.agents/README.md`, `.agents/CONTRACTS.md`, and `.agents/JOBS.md`.

## Architecture rules

- Read `docs/ARCHITECTURE.md` before changing product structure.
- One owner exists for each mechanism. Extend that owner; do not duplicate it in a feature.
- Product code uses feature facades and typed contracts. Product code does not render directly or call raw native APIs.
- `flamewm-reactor` owns native event sources and dispatches typed events.
- UI features project state through their feature facade. They do not become a second window manager or state authority.
- `.vendor/**` is reference material only. It is never an active dependency or runtime input.
- Exceptions are explicit path-level entries in `docs/ARCHITECTURE_EXCEPTIONS.toml`; wildcards are forbidden.

## Handoff

Keep changes within assigned paths, preserve unrelated work, and report exact
verification. Do not claim migration or runtime completion from documentation or
static inspection alone.
