\# Rust Systems Complement

Use with \`STEPWISE_CODING_PLANNER.md\` for every \`*.rs\`/\`Cargo*\` handoff. It adds Rust systems contracts and never weakens the generic execution law.

\#\# 7.2 Toolchain and compatibility

Read root \`Cargo.toml\`, each touched crate manifest, lock/config files, and CI/build scripts before naming edition, MSRV, features, profiles, or dependency behavior. Workspace inheritance is not proof that every package inherits it. State current source facts with their path; if edition/MSRV is absent, overridden, or unverified, require an audit instead of asserting one. Keep public API, feature, target, and dependency changes compatible with source-proven toolchain.

\#\# 7.3 Ownership and native resources

Name owner, acquisition point, transfer/borrow boundary, and release order for every file descriptor, socket, X11 connection/window/pixmap/GC, FFI handle, thread, timer, channel, and mapped resource. Use RAII where existing ownership permits it; do not duplicate cleanup authority. Destruction must prevent callbacks, queued work, and dependent resources from observing released native state.

\#\# 7.4 Event loops and concurrency

Identify event-loop owner and every thread/callback boundary. Event handlers perform bounded state transition and dispatch; blocking I/O, waits, joins, and heavy work require a source-proven off-loop path. Define cancellation, shutdown, ordering, stale-event/generation handling, and return-to-owner delivery where asynchronous work changes state. Do not infer \`Send\`, \`Sync\`, or callback-thread safety from type names.

\#\# 7.5 Module and authority split

Keep one state authority per domain. \`lib.rs\`, crate public APIs, and feature boundaries expose typed contracts; platform/FFI/event-source code remains at existing boundary. Trace \`mod\`, \`pub\`, re-exports, crate features, binary/bootstrap wiring, and cross-crate consumers for every moved or changed symbol. Do not make renderer/UI/event adapters a second manager or state store.

\#\# 7.6 Error contracts

For each fallible operation, define error owner, context, propagation boundary, recovery/rollback, and observable result. Preserve source error detail where callers need diagnosis; no swallowed native/protocol/I/O failure, invented fallback, or panic across FFI/event-loop boundary unless current contract explicitly requires it. Cleanup failures must not mask primary failure without explicit source-proven policy.

\#\# 7.7 Performance contracts

Name hot path, allocation/I/O/lock boundary, event rate, and backpressure/coalescing rule when performance-sensitive code changes. Preserve existing batching, cache lifetime/invalidation, bounded queues, and protocol round-trip behavior unless measurements and source contracts justify change. Benchmark/profile commands, target, and threshold are facts only when current project source or user contract supplies them; otherwise plan semantic/build proof, not invented performance claims.

\#\# 7.8 Rust handoff closure

For each Rust change, handoff resolves manifest/features/toolchain, module/export closure, ownership/drop order, event-loop/thread owner, error behavior, and relevant performance constraint. VERIFY checks compiled feature/target surface using source-proven commands and semantic readback of ownership/lifecycle contracts. Native X11 scope additionally loads \`x11-window-manager-engineering\`; current repository architecture remains authoritative for WM, renderer, reactor, and UI ownership.
