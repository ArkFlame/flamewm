//! Public span guards, points, counters, and memory gauges.

use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::span as registry;
use crate::{now_cpu_ns, now_mono_ns};

/// RAII span guard returned by [`start`]. Call [`SpanGuard::stop`] or drop it.
pub struct SpanGuard {
    idx: usize,
    start_wall: u64,
    start_cpu: u64,
    on_stack: bool,
    stopped: bool,
}

impl SpanGuard {
    pub(crate) fn begin(label: &'static str) -> Self {
        let idx = registry::slot_for(label);
        let start_cpu = now_cpu_ns();
        let start_wall = now_mono_ns();
        let on_stack = registry::push(idx, start_wall, start_cpu);
        Self {
            idx,
            start_wall,
            start_cpu,
            on_stack,
            stopped: false,
        }
    }

    fn finish(&mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        let end_wall = now_mono_ns();
        let end_cpu = now_cpu_ns();
        let wall_inclusive = end_wall.saturating_sub(self.start_wall);
        // Thread CPU clock may be unavailable (0); fall back to wall so the
        // span stays measurable but never fabricates CPU time.
        let cpu_inclusive = if self.start_cpu == 0 || end_cpu == 0 {
            0
        } else {
            end_cpu.saturating_sub(self.start_cpu)
        };
        if self.on_stack {
            let (child_wall, child_cpu) = registry::pop()
                .map(|(_, _, _, cw, cc)| (cw, cc))
                .unwrap_or((0, 0));
            let wall_self = wall_inclusive.saturating_sub(child_wall);
            let cpu_self = cpu_inclusive.saturating_sub(child_cpu);
            registry::record(self.idx, wall_inclusive, wall_self, cpu_inclusive, cpu_self);
            registry::add_child_time(wall_inclusive, cpu_inclusive);
        } else {
            registry::record(
                self.idx,
                wall_inclusive,
                wall_inclusive,
                cpu_inclusive,
                cpu_inclusive,
            );
        }
    }

    /// Stop the span and record it.
    pub fn stop(mut self) {
        self.finish();
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        self.finish();
    }
}

/// Start a timed span under a static label.
pub fn start(label: &'static str) -> SpanGuard {
    SpanGuard::begin(label)
}

/// Reusable static-label timing point.
pub struct ProfilePoint {
    label: &'static str,
}

impl ProfilePoint {
    /// Create a point bound to a static label.
    pub fn new(label: &'static str) -> Self {
        Self { label }
    }

    /// Start one timed occurrence.
    pub fn start(&self) -> SpanGuard {
        SpanGuard::begin(self.label)
    }
}

const MAX_COUNTERS: usize = 64;

struct CounterSlot {
    label: RwLock<Option<&'static str>>,
    value: AtomicU64,
}

impl CounterSlot {
    const fn new() -> Self {
        Self {
            label: RwLock::new(None),
            value: AtomicU64::new(0),
        }
    }
}

macro_rules! counter_slots {
    ($($n:expr),*) => {
        [$( { let _ = $n; CounterSlot::new() } ),*]
    };
}

static COUNTERS: [CounterSlot; MAX_COUNTERS] = counter_slots!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63
);

fn counter_slot(label: &'static str) -> usize {
    for (i, slot) in COUNTERS.iter().enumerate() {
        if slot.label.read().map(|g| *g).unwrap_or(None) == Some(label) {
            return i;
        }
    }
    for (i, slot) in COUNTERS.iter().enumerate() {
        let mut guard = slot.label.write().unwrap_or_else(|e| e.into_inner());
        if guard.is_none() {
            *guard = Some(label);
            return i;
        }
        if *guard == Some(label) {
            return i;
        }
    }
    MAX_COUNTERS - 1
}

/// Monotonic event counter under a static label.
pub struct CounterPoint {
    idx: usize,
}

impl CounterPoint {
    /// Bind a counter to a static label.
    pub fn new(label: &'static str) -> Self {
        Self {
            idx: counter_slot(label),
        }
    }

    /// Increment by one.
    pub fn increment(&self) {
        self.increment_by(1);
    }

    /// Increment by `n`.
    pub fn increment_by(&self, n: u64) {
        COUNTERS[self.idx].value.fetch_add(n, Ordering::Relaxed);
    }
}

pub(crate) fn counter_snapshot_and_reset() -> Vec<(&'static str, u64)> {
    let mut out = Vec::new();
    for slot in COUNTERS.iter() {
        if let Some(label) = slot.label.read().map(|g| *g).unwrap_or(None) {
            let v = slot.value.swap(0, Ordering::Relaxed);
            if v > 0 {
                out.push((label, v));
            }
        }
    }
    out
}

#[cfg(test)]
pub(crate) fn counter_reset_for_test() {
    for slot in COUNTERS.iter() {
        slot.value.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_reset_drains() {
        let _guard = crate::span::TEST_LOCK.lock().unwrap();
        counter_reset_for_test();
        let c = CounterPoint::new("t_ctr_reset");
        c.increment_by(7);
        let rows = counter_snapshot_and_reset();
        assert!(rows.iter().any(|(l, v)| *l == "t_ctr_reset" && *v == 7));
        let rows2 = counter_snapshot_and_reset();
        assert!(!rows2.iter().any(|(l, _)| *l == "t_ctr_reset"));
    }
}
