use flamewm_api::Size;
use flamewm_ui_core::style::ShellMetrics;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarMonth {
    Previous,
    Current,
    Next,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarCell {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub month_kind: CalendarMonth,
    pub today: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarGrid {
    pub year: i32,
    pub month: u8,
    pub cells: [CalendarCell; 42],
}
impl CalendarGrid {
    /// UTC-derived convenience only (via `SystemTime` epoch days).
    /// Production UI must pass local y/m/d into [`CalendarGrid::build_for_date`];
    /// never rely on `build()` near local midnight / UTC rollover.
    pub fn build(year: i32, month: u8) -> Option<Self> {
        let (today_year, today_month, today_day) = today_date();
        Self::build_for_date(year, month, today_year, today_month, today_day)
    }
    /// Build a Sunday-first 6x7 grid, marking `today_*` as today.
    /// Contract: callers must pass the *local* calendar date here, so the
    /// `today` cell stays correct across UTC/local midnight rollover.
    /// `chrono` is intentionally not a dependency of shell-core, so no
    /// `local_ymd()` helper lives here; resolve local y/m/d at the shell
    /// boundary and forward it.
    pub fn build_for_date(
        year: i32,
        month: u8,
        today_year: i32,
        today_month: u8,
        today_day: u8,
    ) -> Option<Self> {
        if !(1..=12).contains(&month) {
            return None;
        }
        let days = days_in_month(year, month);
        let first = sunday_index(year, month, 1);
        let (previous_year, previous_month) = previous_month(year, month);
        let (next_year, next_month) = next_month(year, month);
        let previous_days = days_in_month(previous_year, previous_month);
        let mut cells = Vec::with_capacity(42);
        for index in 0..42 {
            let relative_day = index as i32 - i32::from(first) + 1;
            let (cell_year, cell_month, day, month_kind) = if relative_day < 1 {
                (
                    previous_year,
                    previous_month,
                    (i32::from(previous_days) + relative_day) as u8,
                    CalendarMonth::Previous,
                )
            } else if relative_day > i32::from(days) {
                (
                    next_year,
                    next_month,
                    (relative_day - i32::from(days)) as u8,
                    CalendarMonth::Next,
                )
            } else {
                (year, month, relative_day as u8, CalendarMonth::Current)
            };
            cells.push(CalendarCell {
                year: cell_year,
                month: cell_month,
                day,
                month_kind,
                today: cell_year == today_year && cell_month == today_month && day == today_day,
            });
        }
        Some(Self {
            year,
            month,
            cells: cells.try_into().ok().expect("calendar has 42 cells"),
        })
    }
}
fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}
fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}
fn monday_index(year: i32, month: u8, day: u8) -> u8 {
    let mut y = i64::from(year);
    let mut m = i64::from(month);
    if m < 3 {
        y -= 1;
        m += 12;
    }
    let k = y.rem_euclid(100);
    let j = y.div_euclid(100);
    let h = (i64::from(day) + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 + 5 * j).rem_euclid(7);
    ((h + 5).rem_euclid(7)) as u8
}
fn sunday_index(year: i32, month: u8, day: u8) -> u8 {
    (monday_index(year, month, day) + 1) % 7
}
fn previous_month(year: i32, month: u8) -> (i32, u8) {
    if month == 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    }
}
fn next_month(year: i32, month: u8) -> (i32, u8) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}
fn today_date() -> (i32, u8, u8) {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    civil_date_from_days((seconds / 86_400) as i64)
}
fn civil_date_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    (year as i32 + (month <= 2) as i32, month as u8, day as u8)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockFormat {
    pub time: &'static str,
    pub date: &'static str,
}
impl Default for ClockFormat {
    fn default() -> Self {
        Self {
            time: "%H:%M",
            date: "%-d/%-m/%y",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockTimerPlan {
    pub interval_ms: u64,
    pub initial_delay_ms: u64,
}
#[must_use]
pub fn clock_timer_plan(format: &str, now_epoch_ms: u64) -> ClockTimerPlan {
    let with_seconds = clock_shows_seconds(format);
    let interval_ms = if with_seconds { 1_000 } else { 60_000 };
    ClockTimerPlan {
        interval_ms,
        initial_delay_ms: wall_clock_delay_ms(now_epoch_ms, with_seconds),
    }
}
#[must_use]
pub fn clock_shows_seconds(format: &str) -> bool {
    ["%S", "%T", "%X", "%r"]
        .iter()
        .any(|token| format.contains(token))
}
/// Milliseconds until the next second (`with_seconds == true`) or minute
/// boundary after `now_epoch_ms`. Mirrors the `flamewm-reactor` wall-clock
/// primitive so shell-core stays dependency-free; recompute after every tick
/// so suspend/resume jumps realign instead of drifting. Returns the full
/// interval when exactly on a boundary so the timer always advances.
#[must_use]
pub fn wall_clock_delay_ms(now_epoch_ms: u64, with_seconds: bool) -> u64 {
    let interval_ms = if with_seconds { 1_000 } else { 60_000 };
    let remainder = now_epoch_ms % interval_ms;
    if remainder == 0 {
        interval_ms
    } else {
        interval_ms - remainder
    }
}
/// Minimal wall-clock consumer: remembers the last projected local date and
/// reports when the calendar must be rebuilt. The reactor fires the tick;
/// this struct decides date/calendar refresh on local-date change only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockDateTracker {
    last_date: Option<(i32, u8, u8)>,
}
impl ClockDateTracker {
    #[must_use]
    pub fn new(initial_date: (i32, u8, u8)) -> Self {
        Self {
            last_date: Some(initial_date),
        }
    }
    /// Returns `true` when `today` differs from the last seen local date and
    /// stores it. Midnight/DST/suspend jumps surface as a single change.
    pub fn date_changed(&mut self, today: (i32, u8, u8)) -> bool {
        if self.last_date == Some(today) {
            false
        } else {
            self.last_date = Some(today);
            true
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    struct FakeClock {
        now_ms: std::cell::Cell<u64>,
    }

    impl FakeClock {
        fn new(now_ms: u64) -> Self {
            Self {
                now_ms: std::cell::Cell::new(now_ms),
            }
        }

        fn advance(&self, delta_ms: u64) {
            self.now_ms.set(self.now_ms.get() + delta_ms);
        }

        fn now_epoch_ms(&self) -> u64 {
            self.now_ms.get()
        }
    }

    #[test]
    fn wall_clock_delay_aligns_to_live_boundaries() {
        assert_eq!(wall_clock_delay_ms(61_234, false), 58_766);
        assert_eq!(wall_clock_delay_ms(1_250, true), 750);
        assert_eq!(wall_clock_delay_ms(60_000, false), 60_000);
        assert_eq!(wall_clock_delay_ms(1_000, true), 1_000);
    }

    #[test]
    fn wall_clock_delay_realigns_after_suspend_jump_without_sleep() {
        // No `Thread::sleep` anywhere: the fake clock jumps instantly.
        let clock = FakeClock::new(61_234);
        assert_eq!(wall_clock_delay_ms(clock.now_epoch_ms(), false), 58_766);
        clock.advance(3_630_000);
        assert_eq!(wall_clock_delay_ms(clock.now_epoch_ms(), false), 28_766);
    }

    #[test]
    fn date_tracker_fires_once_per_local_date_change() {
        let mut tracker = ClockDateTracker::new((2026, 9, 5));
        assert!(!tracker.date_changed((2026, 9, 5)));
        assert!(tracker.date_changed((2026, 9, 6)));
        assert!(!tracker.date_changed((2026, 9, 6)));
        // Suspend jump across several days surfaces as a single change.
        assert!(tracker.date_changed((2026, 9, 9)));
        assert!(!tracker.date_changed((2026, 9, 9)));
    }
}

#[must_use]
pub fn calendar_popover_size(metrics: ShellMetrics) -> Size {
    Size::new(
        i32::from(metrics.clock_popover_width),
        i32::from(metrics.popover_padding) * 2 + 7 * 32,
    )
}
