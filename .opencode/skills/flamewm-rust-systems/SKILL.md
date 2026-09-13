---
name: flamewm-rust-systems
description: Implement Rust systems changes with explicit ownership, events, lifetimes, and native boundaries.
---

Keep ownership, event ordering, cancellation, and teardown explicit. Avoid hidden shared mutable state and lifetime extension. Keep unsafe code out of workspace-inherited lint domains; confine required native FFI to designated boundary crates with clear ownership and destruction rules. Use typed contracts between product mechanisms.
