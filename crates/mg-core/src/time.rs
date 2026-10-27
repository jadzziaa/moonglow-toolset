//! Calendar dates from the clock, in UTC: what the toolset writes into
//! files and logs (a claim's date, a script's header, an ERF's build date,
//! a debug log's times). Five copies of the day-to-date arithmetic were
//! one each before.

/// Seconds since 1970 now (0 for a clock set before it).
pub fn unix_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The day `days` after 1970-01-01 as (year, month, day) of the proleptic
/// Gregorian calendar (Howard Hinnant's `civil_from_days`).
pub fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

/// The days from 1970-01-01 to a date ([`civil`] the other way: Hinnant's
/// `days_from_civil`).
pub fn days(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((i64::from(month) + 9) % 12) + 2) / 5 + i64::from(day) - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// The date of a time in seconds since 1970: `2026-10-22`.
pub fn date(secs: i64) -> String {
    let (y, m, d) = civil(secs.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// The time of day of a time in seconds since 1970: `15:59:03`.
pub fn time_of_day(secs: i64) -> String {
    let s = secs.rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// Today's date: `2026-10-22`.
pub fn today() -> String {
    date(unix_now() as i64)
}

/// A time in seconds since 1970 as an ERF header's build date: years since
/// 1900 and the day of the year (from 0).
pub fn year_day(secs: u64) -> (u32, u32) {
    let at = (secs / 86_400) as i64;
    let (year, _, _) = civil(at);
    ((year - 1900) as u32, (at - days(year, 1, 1)) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_from_seconds_and_back() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(1_792_627_200), "2026-10-22");
        assert_eq!(date(951_782_400), "2000-02-29");
        assert_eq!(date(-86_400), "1969-12-31");
        assert_eq!(time_of_day(1_792_627_200 + 57_543), "15:59:03");
        // Every day of four centuries goes there and back.
        for d in -146_097..2 * 146_097 {
            let (y, m, day) = civil(d);
            assert_eq!(days(y, m, day), d);
        }
        // An ERF's build date.
        assert_eq!(year_day(0), (70, 0));
        // 2026-09-30T00:00:00Z is day 272 (0-based) of 2026.
        assert_eq!(year_day(1_790_726_400), (126, 272));
        // 2024-12-31 is day 365 of a leap year.
        assert_eq!(year_day(1_735_603_200), (124, 365));
    }
}
