//! Aggregate rows derived from span/counter snapshots.

use crate::span::Snapshot;

/// One aggregated span row (one report window).
pub struct SpanAgg {
    pub label: &'static str,
    pub calls: u64,
    pub total_wall: u64,
    pub self_wall: u64,
    pub max_wall: u64,
    pub p95_wall: u64,
    pub total_cpu: u64,
    pub self_cpu: u64,
    pub max_cpu: u64,
    pub over_16ms: u64,
}

impl SpanAgg {
    pub(crate) fn from_snapshot(s: &Snapshot) -> Self {
        Self {
            label: s.label,
            calls: s.calls,
            total_wall: s.total_wall,
            self_wall: s.self_wall,
            max_wall: s.max_wall,
            p95_wall: s.p95_wall_ns(),
            total_cpu: s.total_cpu,
            self_cpu: s.self_cpu,
            max_cpu: s.max_cpu,
            over_16ms: s.over_16ms,
        }
    }

    /// One-core CPU percent over `window_ns`: 100% = one core saturated.
    pub(crate) fn cpu_percent_one_core(&self, window_ns: u64) -> f64 {
        crate::process::cpu_percent(self.total_cpu, window_ns)
    }
}

fn tie_label(a: &SpanAgg, b: &SpanAgg) -> std::cmp::Ordering {
    a.label.cmp(b.label)
}

/// Snapshot-and-reset span aggregates with deterministic tie-breaks.
pub fn span_aggs_reset() -> Vec<SpanAgg> {
    crate::span::snapshot_and_reset()
        .iter()
        .map(SpanAgg::from_snapshot)
        .collect()
}

/// Slowest single (wall max desc, tie: label asc) — slow wall ranks above
/// low-CPU work on purpose: wall is what the user feels.
pub fn by_slowest_single(mut rows: Vec<SpanAgg>) -> Vec<SpanAgg> {
    rows.sort_by(|a, b| b.max_wall.cmp(&a.max_wall).then_with(|| tie_label(a, b)));
    rows
}

/// Highest total wall desc, tie: label asc.
pub fn by_highest_total(mut rows: Vec<SpanAgg>) -> Vec<SpanAgg> {
    rows.sort_by(|a, b| {
        b.total_wall
            .cmp(&a.total_wall)
            .then_with(|| tie_label(a, b))
    });
    rows
}

/// Highest call count desc, tie: label asc.
pub fn by_highest_count(mut rows: Vec<SpanAgg>) -> Vec<SpanAgg> {
    rows.sort_by(|a, b| b.calls.cmp(&a.calls).then_with(|| tie_label(a, b)));
    rows
}

/// Collect counter aggregates (snapshot-and-reset) sorted by value desc.
pub fn counter_aggs_reset() -> Vec<(&'static str, u64)> {
    let mut rows = crate::point::counter_snapshot_and_reset();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span;

    #[test]
    fn slowest_ranks_slow_wall_above_low_cpu() {
        let _guard = crate::span::TEST_LOCK.lock().unwrap();
        span::reset_for_test();
        // Slow wall, almost no CPU (blocked) must outrank CPU-heavy fast span.
        span::fake_complete("t_st_blocked", 50_000_000, 1_000);
        span::fake_complete("t_st_busy", 2_000_000, 1_900_000);
        let rows: Vec<SpanAgg> = span::peek().iter().map(SpanAgg::from_snapshot).collect();
        let ranked = by_slowest_single(rows);
        assert_eq!(ranked[0].label, "t_st_blocked");
        span::reset_for_test();
    }

    #[test]
    fn tie_break_is_label_asc() {
        let _guard = crate::span::TEST_LOCK.lock().unwrap();
        span::reset_for_test();
        span::fake_complete("t_tb_b", 10, 10);
        span::fake_complete("t_tb_a", 10, 10);
        let rows: Vec<SpanAgg> = span::peek().iter().map(SpanAgg::from_snapshot).collect();
        let ranked = by_slowest_single(rows);
        assert_eq!(ranked[0].label, "t_tb_a");
        assert_eq!(ranked[1].label, "t_tb_b");
        span::reset_for_test();
    }

    #[test]
    fn windows_reset_after_drain() {
        let _guard = crate::span::TEST_LOCK.lock().unwrap();
        span::reset_for_test();
        crate::point::counter_reset_for_test();
        span::fake_complete("t_wd", 7, 7);
        crate::CounterPoint::new("t_wd_c").increment();
        assert!(!span_aggs_reset().is_empty());
        assert!(span_aggs_reset().is_empty());
        assert_eq!(counter_aggs_reset(), vec![("t_wd_c", 1)]);
        assert!(counter_aggs_reset().is_empty());
    }
}
