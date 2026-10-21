//! UTC ISO-8601 timestamps (`2026-09-21T12:51:01.713Z`) and seed-id
//! randomness, std-only.

use std::fs::File;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

/// The current UTC time as sd writes it: millisecond precision, `Z`.
pub(crate) fn now_iso() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_millis();
    let secs = i64::try_from(millis / 1000).expect("clock fits in i64");
    let millis_part = u32::try_from(millis % 1000).expect("sub-second fits");
    let days = secs.div_euclid(86_400);
    let second_of_day = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second) = (
        second_of_day / 3600,
        (second_of_day % 3600) / 60,
        second_of_day % 60,
    );
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis_part:03}Z")
}

/// The current UTC date as sd's sync commit message uses it
/// (`seeds: sync 2026-09-22`).
pub(crate) fn today_utc() -> String {
    now_iso()
        .split('T')
        .next()
        .expect("now_iso always carries a 'T' date separator")
        .to_owned()
}

/// Howard Hinnant's civil-from-days algorithm.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Four lowercase hex digits from the OS entropy pool, falling back to a
/// time-derived value when `/dev/urandom` is unavailable.
pub(crate) fn random_hex4() -> String {
    let mut bytes = [0_u8; 2];
    let drawn = File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut bytes));
    if drawn.is_err() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .subsec_nanos();
        bytes.copy_from_slice(&nanos.to_le_bytes()[..2]);
    }
    format!("{:02x}{:02x}", bytes[0], bytes[1])
}

/// Howard Hinnant's days-from-civil algorithm (the inverse of
/// `civil_from_days`): the epoch day number of a proleptic Gregorian
/// calendar date.
pub(crate) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let shifted_year = if month <= 2 { year - 1 } else { year };
    let era = if shifted_year >= 0 {
        shifted_year
    } else {
        shifted_year - 399
    } / 400;
    let year_of_era = shifted_year - era * 400;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Parses a UTC instant to epoch milliseconds: an sd timestamp
/// (`2026-09-21T12:51:01.713Z`, with `T`-part and fraction optional)
/// or a bare `YYYY-MM-DD` date (midnight UTC). `None` for anything
/// else, including calendar-invalid dates.
pub(crate) fn parse_iso_millis(text: &str) -> Option<i64> {
    let (date_text, time_text) = match text.split_once('T') {
        Some((date, time)) => (date, Some(time)),
        None => (text, None),
    };
    let mut date_parts = date_text.split('-');
    let year = date_parts.next()?.parse::<i64>().ok()?;
    let month = date_parts.next()?.parse::<i64>().ok()?;
    let day = date_parts.next()?.parse::<i64>().ok()?;
    if date_parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    // Round-trip rejects calendar-invalid dates (2026-02-30).
    if civil_from_days(days) != (year, month, day) {
        return None;
    }
    let mut millis = days * 86_400_000;
    if let Some(time) = time_text {
        let stamped = time.strip_suffix('Z')?;
        let (hms, fraction) = match stamped.split_once('.') {
            Some((hms, fraction)) => (hms, fraction),
            None => (stamped, ""),
        };
        let mut parts = hms.splitn(3, ':');
        let hour = parts.next()?.parse::<i64>().ok()?;
        let minute = parts.next()?.parse::<i64>().ok()?;
        let second = parts.next()?.parse::<i64>().ok()?;
        if parts.next().is_some()
            || !(0..24).contains(&hour)
            || !(0..60).contains(&minute)
            || !(0..60).contains(&second)
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        let fraction = &fraction[..fraction.len().min(3)];
        let mut fraction_millis = if fraction.is_empty() {
            0
        } else {
            fraction.parse().ok()?
        };
        for _ in fraction.len()..3 {
            fraction_millis *= 10;
        }
        millis += hour * 3_600_000 + minute * 60_000 + second * 1000 + fraction_millis;
    }
    Some(millis)
}

/// The `YYYY-MM-DD` calendar date (UTC) of an epoch day number.
pub(crate) fn date_string(epoch_days: i64) -> String {
    let (year, month, day) = civil_from_days(epoch_days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// The epoch day of the Monday of the ISO week containing
/// `epoch_days` (weeks start Monday, UTC).
pub(crate) fn monday_of_week(epoch_days: i64) -> i64 {
    // 1970-01-01 was a Thursday: epoch day 0 sits 3 days past that
    // week's Monday.
    epoch_days - (epoch_days + 3).rem_euclid(7)
}

/// The ISO-8601 week label (`2026-W38`) of an epoch day: a week
/// belongs to the year of its Thursday.
pub(crate) fn iso_week_label(epoch_days: i64) -> String {
    let weekday = (epoch_days + 3).rem_euclid(7);
    let thursday = epoch_days + (3 - weekday);
    let (year, _, _) = civil_from_days(thursday);
    let new_year = days_from_civil(year, 1, 1);
    let week = (thursday - new_year) / 7 + 1;
    format!("{year:04}-W{week:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_instant_shape() {
        let now = now_iso();
        assert_eq!(now.len(), 24, "shape: 2026-09-21T12:51:01.713Z");
        assert!(now.ends_with('Z'));
        assert_eq!(&now[10..11], "T");
        assert_eq!(&now[19..20], ".");
    }

    #[test]
    fn civil_dates_match_calendar() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(20_651), (2026, 7, 17));
    }

    #[test]
    fn days_from_civil_round_trips_civil_from_days() {
        for days in [-100_000, -1, 0, 1, 19_723, 20_651, 100_000] {
            let date = civil_from_days(days);
            assert_eq!(days_from_civil(date.0, date.1, date.2), days);
        }
    }

    #[test]
    fn parses_sd_timestamps_and_bare_dates() {
        assert_eq!(parse_iso_millis("1970-01-01T00:00:00.000Z"), Some(0));
        let midnight = parse_iso_millis("2026-09-21").expect("bare date parses");
        assert_eq!(midnight, days_from_civil(2026, 9, 21) * 86_400_000);
        assert_eq!(
            parse_iso_millis("2026-09-21T12:51:01.713Z"),
            Some(midnight + 46_261_713)
        );
        assert_eq!(
            parse_iso_millis("2026-09-21T00:00:01Z"),
            Some(midnight + 1000)
        );
    }

    #[test]
    fn rejects_malformed_and_calendar_invalid_timestamps() {
        for text in [
            "",
            "not a date",
            "2026-13-01",
            "2026-02-30",
            "2026-09-31T00:00:00Z",
            "2026-09-21T24:00:00Z",
            "2026-09-21T12:60:00Z",
            "2026-09-21T12:00:60Z",
            "2026-09-21T12:00:00Zulu",
            "2026-09-21T12:00:00.12x3Z",
        ] {
            assert!(parse_iso_millis(text).is_none(), "rejects {text:?}");
        }
    }

    #[test]
    fn iso_weeks_follow_the_thursday_rule() {
        // 1970-01-01 was a Thursday: week 1 of 1970.
        assert_eq!(iso_week_label(0), "1970-W01");
        // 2026-01-01 is a Thursday; 2025-12-29 (Monday) belongs to it.
        let new_year = days_from_civil(2026, 1, 1);
        assert_eq!(iso_week_label(new_year), "2026-W01");
        assert_eq!(iso_week_label(new_year - 3), "2026-W01");
        // The repo's burst week: 2026-09-21 (Monday) is 2026-W39.
        assert_eq!(iso_week_label(days_from_civil(2026, 9, 21)), "2026-W39");
    }

    #[test]
    fn monday_of_week_lands_on_a_monday() {
        for days in [0, 3, 6, 13, 20_651] {
            let monday = monday_of_week(days);
            assert_eq!((monday + 3).rem_euclid(7), 0);
            assert!(monday <= days && days < monday + 7);
        }
    }

    #[test]
    fn hex4_is_four_lowercase_hex_digits() {
        for _ in 0..16 {
            let hex = random_hex4();
            assert_eq!(hex.len(), 4);
            assert!(
                hex.bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            );
        }
    }
}
