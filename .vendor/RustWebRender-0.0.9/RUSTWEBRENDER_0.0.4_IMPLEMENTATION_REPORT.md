# RustWebRender 0.0.4 Implementation Report

## Scope

0.0.4 is a launcher-lifecycle repair release. It keeps the 0.0.3 FlameWM HTML/CSS renderer, RWRB/2 document format, Xft/IBM Plex Sans path, images, cursor handling and interactive prototype behavior unchanged unless required to make the default `./xephyr` workflow reliable.

## Reproduced failure mechanism

The 0.0.3 interactive launcher started the nested server as:

```text
Xephyr :97 ... -reset -terminate
```

It then probed readiness using a short-lived client:

```text
DISPLAY=:97 xdpyinfo
```

`-terminate` is an Xserver option that makes the server terminate at server reset. When the readiness client disconnected before the Rust renderer had connected, the nested X server was allowed to reset and terminate. The next operation therefore failed in the renderer's `XOpenDisplay(NULL)` call even though the shell correctly supplied `DISPLAY=:97` to the process.

This explains the exact observed sequence:

```text
black Xephyr briefly appears
-> readiness client disconnects
-> Xephyr exits
-> rwr-flamewm-demo starts
-> XOpenDisplay fails because :97 no longer exists
```

The visual verifier did not have the same failure because it did not start its Xephyr with `-terminate`, which is why the full prototype could be seen there briefly before the verifier intentionally cleaned up its temporary display.

## Fix

The interactive launcher now:

1. resolves and enters the package root;
2. prepares IBM Plex Sans if necessary;
3. builds `rwr-compile` and `rwr-flamewm-demo` before creating any nested display;
4. compiles the FlameWM prototype to `target/rwr/flamewm-v8.rwr`;
5. chooses a genuinely free display (`:97..:119`) or validates the explicit override;
6. starts Xephyr with `-noreset` and without `-terminate`;
7. waits for `xdpyinfo` readiness while requiring the spawned Xephyr PID to remain alive;
8. waits again after the readiness-client disconnect and rejects a server that did not persist;
9. runs `rwr-flamewm-demo` in the foreground with the nested `DISPLAY`;
10. keeps Xephyr alive until the user closes it, closes the demo, or presses Ctrl+C;
11. cleans up the exact spawned Xephyr PID on exit.

The nested server also disables TCP listening because the prototype only needs local Unix-domain X11 transport.

## Verification performed in the packaging environment

The packaging environment does not contain Xephyr or Cargo, so it cannot execute the complete Rust GUI path. It does contain Xvfb, which uses the same Xserver reset options. The failure mechanism was directly reproduced there:

```text
-terminate -> readiness probe succeeds -> server is dead after client disconnect
-noreset   -> readiness probe succeeds -> server remains alive after client disconnect
```

The 0.0.4 launcher was also shell-syntax checked and exercised with bounded command shims to verify that it builds before server startup, supplies the nested `DISPLAY` to the demo, requires Xephyr liveness at readiness, and contains `-noreset` with no `-terminate`.

The user's 0.0.3 logs independently prove that the Rust workspace itself builds and all 12 current unit tests pass on the target machine.

## Manual target-machine gate

```bash
./doctor
./check
./xephyr
```

Expected behavior of `./xephyr`:

- one Xephyr window opens;
- the FlameWM prototype appears and stays visible;
- mouse interaction remains available indefinitely;
- terminal remains attached to `rwr-flamewm-demo`;
- closing Xephyr or pressing Ctrl+C exits and cleans up the nested server.

`./verify-visual` remains a separate ephemeral capture/comparison command and is expected to close its temporary Xephyr automatically.
