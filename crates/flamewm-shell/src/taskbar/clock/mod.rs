use flamewm_shell_core::{CalendarGrid, ClockFormat, ClockTimerPlan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockView {
    pub text: String,
    pub format: ClockFormat,
}

#[must_use]
pub fn timer_plan(format: &str, now_epoch_ms: u64) -> ClockTimerPlan {
    flamewm_shell_core::clock_timer_plan(format, now_epoch_ms)
}

#[must_use]
pub fn calendar(year: i32, month: u8) -> Option<CalendarGrid> {
    CalendarGrid::build(year, month)
}
