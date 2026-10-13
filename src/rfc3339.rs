//! UTC RFC 3339 formatting for [`Wall`](crate::Wall), with no dependencies
//! and no allocation.
//!
//! The calendar conversion is Howard Hinnant's `civil_from_days` for the
//! proleptic Gregorian calendar
//! (<https://howardhinnant.github.io/date_algorithms.html>). It is exact for
//! every day count the crate can produce. Unix time does not count leap
//! seconds, so the seconds field is always `00`–`59`.

use core::fmt::{self, Write as _};

const SECS_PER_DAY: i128 = 86_400;

/// Divisors that keep the leading `n` fractional digits of a nanosecond
/// count, indexed by `n` (0..=9).
const FRACTION_SCALE: [u32; 10] = [
    1_000_000_000,
    100_000_000,
    10_000_000,
    1_000_000,
    100_000,
    10_000,
    1_000,
    100,
    10,
    1,
];

/// Longest output: a 12-digit signed year, `-MM-DDTHH:MM:SS`, nine
/// fractional digits and `Z` is 39 bytes.
const MAX_LEN: usize = 48;

/// Converts a day count relative to 1970-01-01 to a proleptic Gregorian
/// `(year, month, day)`, with `month` in `1..=12` and `day` in `1..=31`.
pub(crate) fn civil_from_days(days: i128) -> (i128, i128, i128) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i128::from(month <= 2);
    (year, month, day)
}

/// Number of fractional digits used when the caller gives no precision:
/// the shortest of 0, 3, 6 or 9 that represents `nanos` exactly.
fn auto_fraction_digits(nanos: u32) -> usize {
    if nanos == 0 {
        0
    } else if nanos % 1_000_000 == 0 {
        3
    } else if nanos % 1_000 == 0 {
        6
    } else {
        9
    }
}

/// Writes `secs` (floor Unix seconds) plus `nanos` (`0..1_000_000_000`) as
/// an RFC 3339 UTC timestamp with exactly `digits` fractional digits
/// (truncated, never rounded, so the output never moves into the next
/// second).
pub(crate) fn write_timestamp(
    out: &mut impl fmt::Write,
    secs: i128,
    nanos: u32,
    digits: usize,
) -> fmt::Result {
    let days = secs.div_euclid(SECS_PER_DAY);
    let second_of_day = secs.rem_euclid(SECS_PER_DAY);
    let (year, month, day) = civil_from_days(days);

    if (0..=9_999).contains(&year) {
        write!(out, "{year:04}")?;
    } else {
        // ISO 8601 expanded representation; outside RFC 3339 proper.
        write!(out, "{year:+05}")?;
    }
    write!(
        out,
        "-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        second_of_day / 3_600,
        second_of_day % 3_600 / 60,
        second_of_day % 60
    )?;

    let digits = digits.min(9);
    if digits > 0 {
        let fraction = nanos / FRACTION_SCALE[digits];
        write!(out, ".{fraction:0digits$}")?;
    }
    out.write_char('Z')
}

/// Formats through `f`, honouring its precision (fractional digits) and its
/// width, fill and alignment.
pub(crate) fn fmt(f: &mut fmt::Formatter<'_>, secs: i128, nanos: u32) -> fmt::Result {
    let digits = f.precision().unwrap_or_else(|| auto_fraction_digits(nanos));
    let mut buf = StackBuf::new();
    write_timestamp(&mut buf, secs, nanos, digits)?;
    pad(f, buf.as_str()?)
}

/// Applies width, fill and alignment without `Formatter::pad`, which would
/// reinterpret the precision as a maximum string length.
fn pad(f: &mut fmt::Formatter<'_>, text: &str) -> fmt::Result {
    let fill_len = f
        .width()
        .map_or(0, |width| width.saturating_sub(text.len()));
    if fill_len == 0 {
        return f.write_str(text);
    }
    let (before, after) = match f.align() {
        Some(fmt::Alignment::Right) => (fill_len, 0),
        Some(fmt::Alignment::Center) => (fill_len / 2, fill_len - fill_len / 2),
        Some(fmt::Alignment::Left) | None => (0, fill_len),
    };
    let fill = f.fill();
    for _ in 0..before {
        f.write_char(fill)?;
    }
    f.write_str(text)?;
    for _ in 0..after {
        f.write_char(fill)?;
    }
    Ok(())
}

/// A fixed-capacity ASCII buffer, so formatting never allocates.
struct StackBuf {
    bytes: [u8; MAX_LEN],
    len: usize,
}

impl StackBuf {
    const fn new() -> Self {
        Self {
            bytes: [0; MAX_LEN],
            len: 0,
        }
    }

    fn as_str(&self) -> Result<&str, fmt::Error> {
        self.bytes
            .get(..self.len)
            .and_then(|filled| core::str::from_utf8(filled).ok())
            .ok_or(fmt::Error)
    }
}

impl fmt::Write for StackBuf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len.checked_add(s.len()).ok_or(fmt::Error)?;
        let dst = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
        dst.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(secs: i128, nanos: u32, digits: usize) -> String {
        let mut out = String::new();
        assert!(write_timestamp(&mut out, secs, nanos, digits).is_ok());
        out
    }

    #[test]
    fn test_civil_from_days_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(civil_from_days(-719_468), (0, 3, 1));
        assert_eq!(civil_from_days(-719_162), (1, 1, 1));
        assert_eq!(civil_from_days(2_932_896), (9_999, 12, 31));
    }

    /// Exhaustive: every day of two full 400-year Gregorian cycles
    /// (1600-03-01 to 2400-02-29), crossing the epoch, against a naive
    /// day-by-day calendar counter.
    #[test]
    fn test_civil_from_days_matches_naive_calendar_for_two_eras() {
        fn days_in_month(year: i128, month: i128) -> i128 {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            match month {
                2 if leap => 29,
                2 => 28,
                4 | 6 | 9 | 11 => 30,
                _ => 31,
            }
        }
        let first = civil_from_days(-135_080);
        assert_eq!(first, (1600, 3, 1));
        let (mut year, mut month, mut day) = first;
        for days in -135_080..(-135_080 + 2 * 146_097) {
            assert_eq!(civil_from_days(days), (year, month, day), "day {days}");
            day += 1;
            if day > days_in_month(year, month) {
                day = 1;
                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
            }
        }
        assert_eq!((year, month, day), (2400, 3, 1));
    }

    #[test]
    fn test_write_timestamp_epoch_and_digit_counts() {
        assert_eq!(render(0, 0, 0), "1970-01-01T00:00:00Z");
        assert_eq!(render(0, 123_456_789, 3), "1970-01-01T00:00:00.123Z");
        assert_eq!(render(0, 123_456_789, 9), "1970-01-01T00:00:00.123456789Z");
        assert_eq!(render(0, 5, 9), "1970-01-01T00:00:00.000000005Z");
        assert_eq!(render(0, 999_999_999, 1), "1970-01-01T00:00:00.9Z");
        assert_eq!(render(0, 1, 42), "1970-01-01T00:00:00.000000001Z");
    }

    #[test]
    fn test_write_timestamp_negative_seconds_floor_correctly() {
        assert_eq!(render(-1, 0, 0), "1969-12-31T23:59:59Z");
        assert_eq!(render(-1, 500_000_000, 3), "1969-12-31T23:59:59.500Z");
        assert_eq!(render(-86_400, 0, 0), "1969-12-31T00:00:00Z");
    }

    #[test]
    fn test_write_timestamp_years_outside_four_digits_use_expanded_form() {
        // 10000-01-01T00:00:00Z
        assert_eq!(render(253_402_300_800, 0, 0), "+10000-01-01T00:00:00Z");
        // 0000-01-01 is still four digits; -0001-12-31 needs the sign.
        assert_eq!(render(-62_167_219_200, 0, 0), "0000-01-01T00:00:00Z");
        assert_eq!(render(-62_167_219_201, 0, 0), "-0001-12-31T23:59:59Z");
    }

    #[test]
    fn test_write_timestamp_extreme_inputs_fit_buffer() {
        let extreme = -i128::from(u64::MAX) - 1;
        let mut buf = StackBuf::new();
        assert!(write_timestamp(&mut buf, extreme, 999_999_999, 9).is_ok());
        let mut buf = StackBuf::new();
        assert!(write_timestamp(&mut buf, i128::from(u64::MAX), 999_999_999, 9).is_ok());
        assert!(buf.as_str().is_ok_and(|s| s.ends_with(".999999999Z")));
    }

    #[test]
    fn test_auto_fraction_digits_picks_shortest_exact_width() {
        assert_eq!(auto_fraction_digits(0), 0);
        assert_eq!(auto_fraction_digits(5_000_000), 3);
        assert_eq!(auto_fraction_digits(5_000), 6);
        assert_eq!(auto_fraction_digits(5), 9);
    }

    #[test]
    fn test_stack_buf_rejects_overflow() {
        let mut buf = StackBuf::new();
        let long = "x".repeat(MAX_LEN + 1);
        assert!(buf.write_str(&long).is_err());
        assert!(buf.write_str(&"y".repeat(MAX_LEN)).is_ok());
        assert!(buf.write_str("z").is_err());
    }
}
