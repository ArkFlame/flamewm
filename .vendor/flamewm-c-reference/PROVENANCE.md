# FlameWM C Reference Provenance

Reference-only copy from the supplied broad C corpus formerly at
`.vendor/flamewm-c-original/`.

## Included

- `src/**`: exact bytes copied from the broad C reference source tree.
- `src/.deps/**` is absent because it contained generated dependency metadata,
  not source.

The copied closure is intentionally limited to the C implementation source
tree. It is not a build input for this Rust workspace.

## Absent paths

These paths existed in the broad corpus but are not part of this curated
closure:

- top-level build, Autotools, documentation, tests, tools, `po`, `m4`, `man`,
  and `scripts` trees;
- `flamewm/assets/**`, `flamewm/prototype/**`, `flamewm/reference/**`, and
  `flamewm/**` planning/archive trees;
- `lib/**`, including packaged themes, icons, cursors, and fonts;
- generated `src/.deps/**` files;
- the original broad-corpus archive (not present as a supplied archive in this
  workspace).

No absent path is reconstructed or substituted.
