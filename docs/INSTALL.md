# Install

FlameWM requires an X11 session and runtime libraries for X11, Xft, and
Fontconfig. It does not start a desktop session or replace an existing display
manager.

Build release binaries separately, then package them:

```bash
mkdir -p .target/release
./scripts/package
```

The command refuses incomplete release directories. It writes reproducible
`flamewm-VERSION-source.tar.gz` and `flamewm-VERSION-linux-x86_64.tar.gz`
archives plus `SHA256SUMS` under `.target/dist`. When `dpkg-deb` is installed,
it also writes `flamewm_VERSION_amd64.deb`.

Install the Debian artifact with your normal package manager, or unpack the
Linux archive under `/` as root. Select `FlameWM` from your display manager's
session chooser, then log in to an X11 session.
