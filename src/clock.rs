//! The clock the island shows while it rests.
//!
//! The widget itself is `ui/widgets/clock.slint`; this module is where its data comes from.
//! The hour is read from Windows for now, which costs no new dependency: when Linux stops being
//! a destination and becomes a target, the one function that reads the time is what gets swapped.
//!
//! Everything that decides *what* is shown is a pure function with a test next to it; only
//! reading the system clock, which cannot be tested without mocking the system, is not.

use std::time::Duration;

use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

/// The hour as the island shows it: `HH:MM`, 24 hours.
pub fn current_time() -> String {
    let now = local_time();
    format_time(now.wHour, now.wMinute)
}

/// How long until the displayed value changes, so the island can sleep exactly that long
/// instead of waking up every second to find out that nothing changed.
pub fn until_next_minute() -> Duration {
    let now = local_time();
    until_next_minute_from(now.wSecond, now.wMilliseconds)
}

fn local_time() -> SYSTEMTIME {
    let mut now: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut now) };
    now
}

fn format_time(hour: u16, minute: u16) -> String {
    format!("{hour:02}:{minute:02}")
}

fn until_next_minute_from(second: u16, millisecond: u16) -> Duration {
    let already_elapsed = second as u64 * 1000 + millisecond as u64;
    Duration::from_millis(60_000 - already_elapsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_the_hour_and_the_minute() {
        assert_eq!(format_time(9, 5), "09:05");
        assert_eq!(format_time(0, 0), "00:00");
        assert_eq!(format_time(23, 59), "23:59");
    }

    #[test]
    fn sleeps_exactly_until_the_minute_changes() {
        assert_eq!(until_next_minute_from(0, 0), Duration::from_millis(60_000));
        assert_eq!(until_next_minute_from(20, 0), Duration::from_millis(40_000));
        assert_eq!(until_next_minute_from(59, 500), Duration::from_millis(500));
        // The timer is armed for a future instant, never for zero.
        assert!(until_next_minute_from(59, 999) > Duration::ZERO);
    }
}
