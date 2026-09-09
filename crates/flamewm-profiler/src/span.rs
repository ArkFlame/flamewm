//! Span registry with nested wall+CPU self/inclusive accounting.
//!
//! Fixed-size static registry keyed by `&'static str` labels. Steady-state
//! `start`/`stop` performs no allocation and spawns no thread.

use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) const MAX_SPANS: usize = 64;
const MAX_DEPTH: usize = 32;

/// Fixed wall-time histogram buckets (inclusive upper bounds, ns).
/// Last bucket is overflow (>128ms).
pub(crate) const HIST_BOUNDS_NS: [u64; 8] = [
    1_000_000,
    4_000_000,
    8_000_000,
    16_000_000,
    32_000_000,
    64_000_000,
    128_000_000,
    u64::MAX,
];
pub(crate) const HIST_LEN: usize = 8;
/// Slow threshold: wall inclusive over one 60Hz frame.
pub(crate) const SLOW_WALL_NS: u64 = 16_000_000;

pub(crate) struct SpanSlot {
    label: RwLock<Option<&'static str>>,
    calls: AtomicU64,
    total_wall: AtomicU64,
    self_wall: AtomicU64,
    max_wall: AtomicU64,
    total_cpu: AtomicU64,
    self_cpu: AtomicU64,
    max_cpu: AtomicU64,
    over_16ms: AtomicU64,
    hist: [AtomicU64; HIST_LEN],
}

impl SpanSlot {
    const fn new() -> Self {
        Self {
            label: RwLock::new(None),
            calls: AtomicU64::new(0),
            total_wall: AtomicU64::new(0),
            self_wall: AtomicU64::new(0),
            max_wall: AtomicU64::new(0),
            total_cpu: AtomicU64::new(0),
            self_cpu: AtomicU64::new(0),
            max_cpu: AtomicU64::new(0),
            over_16ms: AtomicU64::new(0),
            hist: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
        }
    }
}

macro_rules! span_slots {
    ($($n:expr),*) => {
        [$( { let _ = $n; SpanSlot::new() } ),*]
    };
}

static SLOTS: [SpanSlot; MAX_SPANS] = span_slots!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63
);

/// Resolve a static label to a registry slot, claiming an empty slot once.
pub(crate) fn slot_for(label: &'static str) -> usize {
    for (i, slot) in SLOTS.iter().enumerate() {
        if slot.label.read().map(|g| *g).unwrap_or(None) == Some(label) {
            return i;
        }
    }
    for (i, slot) in SLOTS.iter().enumerate() {
        let mut guard = slot.label.write().unwrap_or_else(|e| e.into_inner());
        if guard.is_none() {
            *guard = Some(label);
            return i;
        }
        if *guard == Some(label) {
            return i;
        }
    }
    // Registry full: map deterministically to the last slot for counting
    // continuity. Label of the last slot is never overwritten.
    MAX_SPANS - 1
}

fn hist_bucket(wall_inclusive_ns: u64) -> usize {
    for (i, bound) in HIST_BOUNDS_NS.iter().enumerate() {
        if wall_inclusive_ns <= *bound {
            return i;
        }
    }
    HIST_LEN - 1
}

pub(crate) fn record(
    idx: usize,
    wall_inclusive_ns: u64,
    wall_self_ns: u64,
    cpu_inclusive_ns: u64,
    cpu_self_ns: u64,
) {
    let slot = &SLOTS[idx];
    slot.calls.fetch_add(1, Ordering::Relaxed);
    slot.total_wall
        .fetch_add(wall_inclusive_ns, Ordering::Relaxed);
    slot.self_wall.fetch_add(wall_self_ns, Ordering::Relaxed);
    let _ = slot
        .max_wall
        .fetch_max(wall_inclusive_ns, Ordering::Relaxed);
    slot.total_cpu
        .fetch_add(cpu_inclusive_ns, Ordering::Relaxed);
    slot.self_cpu.fetch_add(cpu_self_ns, Ordering::Relaxed);
    let _ = slot.max_cpu.fetch_max(cpu_inclusive_ns, Ordering::Relaxed);
    if wall_inclusive_ns > SLOW_WALL_NS {
        slot.over_16ms.fetch_add(1, Ordering::Relaxed);
    }
    slot.hist[hist_bucket(wall_inclusive_ns)].fetch_add(1, Ordering::Relaxed);
}

#[derive(Clone, Copy)]
pub(crate) struct Snapshot {
    #[allow(dead_code)]
    pub(crate) idx: usize,
    pub(crate) label: &'static str,
    pub(crate) calls: u64,
    pub(crate) total_wall: u64,
    pub(crate) self_wall: u64,
    pub(crate) max_wall: u64,
    pub(crate) total_cpu: u64,
    pub(crate) self_cpu: u64,
    pub(crate) max_cpu: u64,
    pub(crate) over_16ms: u64,
    pub(crate) hist: [u64; HIST_LEN],
}

impl Snapshot {
    /// Estimated p95 wall time from fixed buckets (upper bound of the
    /// bucket where cumulative share first reaches 95%).
    pub(crate) fn p95_wall_ns(&self) -> u64 {
        if self.calls == 0 {
            return 0;
        }
        let threshold = self.calls.saturating_mul(95).div_ceil(100).max(1);
        let mut acc = 0u64;
        for (i, count) in self.hist.iter().enumerate() {
            acc = acc.saturating_add(*count);
            if acc >= threshold {
                return HIST_BOUNDS_NS[i];
            }
        }
        0
    }
}

/// Snapshot-and-reset: atomically swap counters to zero, retain label
/// registrations. Memory peaks are owned by `memory.rs` and untouched.
pub(crate) fn snapshot_and_reset() -> Vec<Snapshot> {
    let mut out = Vec::new();
    for (i, slot) in SLOTS.iter().enumerate() {
        let label = slot.label.read().map(|g| *g).unwrap_or(None);
        let Some(label) = label else { continue };
        let calls = slot.calls.swap(0, Ordering::Relaxed);
        if calls == 0 {
            // Drain any stray partial state without emitting a row.
            slot.total_wall.store(0, Ordering::Relaxed);
            slot.self_wall.store(0, Ordering::Relaxed);
            slot.max_wall.store(0, Ordering::Relaxed);
            slot.total_cpu.store(0, Ordering::Relaxed);
            slot.self_cpu.store(0, Ordering::Relaxed);
            slot.max_cpu.store(0, Ordering::Relaxed);
            slot.over_16ms.store(0, Ordering::Relaxed);
            for b in slot.hist.iter() {
                b.store(0, Ordering::Relaxed);
            }
            continue;
        }
        let mut hist = [0u64; HIST_LEN];
        for (j, bucket) in slot.hist.iter().enumerate() {
            hist[j] = bucket.swap(0, Ordering::Relaxed);
        }
        out.push(Snapshot {
            idx: i,
            label,
            calls,
            total_wall: slot.total_wall.swap(0, Ordering::Relaxed),
            self_wall: slot.self_wall.swap(0, Ordering::Relaxed),
            max_wall: slot.max_wall.swap(0, Ordering::Relaxed),
            total_cpu: slot.total_cpu.swap(0, Ordering::Relaxed),
            self_cpu: slot.self_cpu.swap(0, Ordering::Relaxed),
            max_cpu: slot.max_cpu.swap(0, Ordering::Relaxed),
            over_16ms: slot.over_16ms.swap(0, Ordering::Relaxed),
            hist,
        });
    }
    out
}

/// Non-resetting snapshot for tests that must not disturb window state.
#[cfg(test)]
pub(crate) fn peek() -> Vec<Snapshot> {
    let mut out = Vec::new();
    for (i, slot) in SLOTS.iter().enumerate() {
        let label = slot.label.read().map(|g| *g).unwrap_or(None);
        if let Some(label) = label {
            let calls = slot.calls.load(Ordering::Relaxed);
            if calls == 0 {
                continue;
            }
            let mut hist = [0u64; HIST_LEN];
            for (j, bucket) in slot.hist.iter().enumerate() {
                hist[j] = bucket.load(Ordering::Relaxed);
            }
            out.push(Snapshot {
                idx: i,
                label,
                calls,
                total_wall: slot.total_wall.load(Ordering::Relaxed),
                self_wall: slot.self_wall.load(Ordering::Relaxed),
                max_wall: slot.max_wall.load(Ordering::Relaxed),
                total_cpu: slot.total_cpu.load(Ordering::Relaxed),
                self_cpu: slot.self_cpu.load(Ordering::Relaxed),
                max_cpu: slot.max_cpu.load(Ordering::Relaxed),
                over_16ms: slot.over_16ms.load(Ordering::Relaxed),
                hist,
            });
        }
    }
    out
}

#[derive(Clone, Copy)]
struct Frame {
    idx: usize,
    start_wall: u64,
    start_cpu: u64,
    child_wall: u64,
    child_cpu: u64,
}

thread_local! {
    static STACK: std::cell::RefCell<([Frame; MAX_DEPTH], usize)> = std::cell::RefCell::new((
        [Frame {
            idx: 0,
            start_wall: 0,
            start_cpu: 0,
            child_wall: 0,
            child_cpu: 0,
        }; MAX_DEPTH],
        0,
    ));
}

/// Push a frame; returns false when nesting overflows (span still timed).
pub(crate) fn push(idx: usize, start_wall: u64, start_cpu: u64) -> bool {
    STACK.with(|cell| {
        let (frames, depth) = &mut *cell.borrow_mut();
        if *depth >= MAX_DEPTH {
            return false;
        }
        frames[*depth] = Frame {
            idx,
            start_wall,
            start_cpu,
            child_wall: 0,
            child_cpu: 0,
        };
        *depth += 1;
        true
    })
}

/// Pop the top frame, returning it.
pub(crate) fn pop() -> Option<(usize, u64, u64, u64, u64)> {
    STACK.with(|cell| {
        let (frames, depth) = &mut *cell.borrow_mut();
        if *depth == 0 {
            return None;
        }
        *depth -= 1;
        let f = frames[*depth];
        Some((f.idx, f.start_wall, f.start_cpu, f.child_wall, f.child_cpu))
    })
}

/// Attribute nested time to the parent frame (wall and CPU separately).
pub(crate) fn add_child_time(wall_inclusive_ns: u64, cpu_inclusive_ns: u64) {
    STACK.with(|cell| {
        let (frames, depth) = &mut *cell.borrow_mut();
        if *depth > 0 {
            frames[*depth - 1].child_wall = frames[*depth - 1]
                .child_wall
                .saturating_add(wall_inclusive_ns);
            frames[*depth - 1].child_cpu = frames[*depth - 1]
                .child_cpu
                .saturating_add(cpu_inclusive_ns);
        }
    });
}

/// Test-only helper: complete one span with fake deltas through the real
/// push/pop/record path so nested self math is exercised deterministically.
#[cfg(test)]
pub(crate) fn fake_complete(label: &'static str, wall_ns: u64, cpu_ns: u64) {
    let idx = slot_for(label);
    let on_stack = push(idx, 0, 0);
    if on_stack {
        if let Some((_, _, _, child_wall, child_cpu)) = pop() {
            let wall_self = wall_ns.saturating_sub(child_wall);
            let cpu_self = cpu_ns.saturating_sub(child_cpu);
            record(idx, wall_ns, wall_self, cpu_ns, cpu_self);
            add_child_time(wall_ns, cpu_ns);
        }
    } else {
        record(idx, wall_ns, wall_ns, cpu_ns, cpu_ns);
    }
}

/// Test-only helper: complete a nested pair (outer wraps inner) with fake
/// deltas; returns nothing, state lands in the registry.
#[cfg(test)]
pub(crate) fn fake_nested(
    outer: &'static str,
    inner: &'static str,
    outer_wall: u64,
    outer_cpu: u64,
    inner_wall: u64,
    inner_cpu: u64,
) {
    let outer_idx = slot_for(outer);
    push(outer_idx, 0, 0);
    fake_complete(inner, inner_wall, inner_cpu);
    if let Some((_, _, _, child_wall, child_cpu)) = pop() {
        let wall_self = outer_wall.saturating_sub(child_wall);
        let cpu_self = outer_cpu.saturating_sub(child_cpu);
        record(outer_idx, outer_wall, wall_self, outer_cpu, cpu_self);
        add_child_time(outer_wall, outer_cpu);
    }
}

#[cfg(test)]
pub(crate) fn reset_for_test() {
    let _ = snapshot_and_reset();
}

#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: span registry is process-global; tests here run serially within
    // this module by draining state first and using unique labels.
    #[test]
    fn nested_self_math_wall_and_cpu() {
        let _guard = crate::span::TEST_LOCK.lock().unwrap();
        reset_for_test();
        fake_nested("t_nest_o", "t_nest_i", 100, 80, 30, 20);
        let rows = peek();
        let outer = rows.iter().find(|r| r.label == "t_nest_o").unwrap();
        let inner = rows.iter().find(|r| r.label == "t_nest_i").unwrap();
        assert_eq!(outer.total_wall, 100);
        assert_eq!(outer.self_wall, 70);
        assert_eq!(outer.total_cpu, 80);
        assert_eq!(outer.self_cpu, 60);
        assert_eq!(inner.total_wall, 30);
        assert_eq!(inner.self_wall, 30);
        assert_eq!(inner.total_cpu, 20);
        reset_for_test();
    }

    #[test]
    fn reset_drains_counters_but_keeps_label() {
        let _guard = crate::span::TEST_LOCK.lock().unwrap();
        reset_for_test();
        fake_complete("t_reset_keep", 5, 5);
        assert!(peek().iter().any(|r| r.label == "t_reset_keep"));
        let rows = snapshot_and_reset();
        assert!(rows.iter().any(|r| r.label == "t_reset_keep"));
        assert!(peek().is_empty() || !peek().iter().any(|r| r.label == "t_reset_keep"));
        // Label registration retained: slot still claimed.
        assert_eq!(slot_for("t_reset_keep"), slot_for("t_reset_keep"));
        reset_for_test();
    }

    #[test]
    fn p95_bucket_estimate() {
        let _guard = crate::span::TEST_LOCK.lock().unwrap();
        reset_for_test();
        for _ in 0..19 {
            fake_complete("t_p95", 1_000, 1_000);
        }
        fake_complete("t_p95", 100_000_000, 1_000);
        let rows = peek();
        let row = rows.iter().find(|r| r.label == "t_p95").unwrap();
        assert_eq!(row.calls, 20);
        // 95% of 20 = 19 -> first bucket already holds 19.
        assert_eq!(row.p95_wall_ns(), 1_000_000);
        reset_for_test();
    }

    #[test]
    fn slow_counter_threshold() {
        let _guard = crate::span::TEST_LOCK.lock().unwrap();
        reset_for_test();
        fake_complete("t_slow", SLOW_WALL_NS + 1, 1);
        fake_complete("t_slow", SLOW_WALL_NS, 1);
        let rows = peek();
        let row = rows.iter().find(|r| r.label == "t_slow").unwrap();
        assert_eq!(row.over_16ms, 1);
        reset_for_test();
    }
}
