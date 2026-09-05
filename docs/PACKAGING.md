# Packaging

`./scripts/package` is packaging-only. It never runs Cargo, compiles sources,
or runs tests. It validates that current executable release artifacts exist in
`.target/release`, copies files according to `packaging/install-manifest.txt`,
and stages `flamewm-VERSION` under `.target/dist`.

Archive metadata is normalized with sorted entries, numeric owner/group zero,
and `SOURCE_DATE_EPOCH` (default `0`). The reproducible outputs are
`flamewm-VERSION-source.tar.gz`, `flamewm-VERSION-linux-x86_64.tar.gz`, and
`SHA256SUMS` under `.target/dist`.

If `dpkg-deb` exists, `flamewm_VERSION_amd64.deb` is emitted beside the
archives. Debian metadata comes from `packaging/debian/control.in` and
`copyright`. Absence of `dpkg-deb` is allowed; missing release binaries or
package sources are errors. `SHA256SUMS` covers every artifact that was
emitted.
