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
