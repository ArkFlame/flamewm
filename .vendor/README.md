# Vendored references

Reference-only material. Production code must not import, link, or load from `.vendor/`.

| Directory | Role |
|---|---|
| `RustWebRender-0.0.9` | Exact RustWebRender 0.0.9 source from the local canonical checkout `ArkFlame/rust-webrender` at commit `3919923ee0b62bc23f2626ca3cbcd69bc63f1f9c` (`origin/master`); preserved without source edits. |
| `flamewm-c-reference` | Curated C implementation source closure. See its `PROVENANCE.md`. |

The RustWebRender vendor contains tracked source/reference assets only. Nested Git metadata and build outputs are intentionally omitted; no build or runtime source lives here.
