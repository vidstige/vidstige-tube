//! Seconds since the epoch to the `YYYYMMDDTHHMMSSZ` form V4 signing uses.

/// Civil date from days since 1970-01-01, after Howard Hinnant's
/// `civil_from_days`. The shift by 719468 moves the epoch to
/// 0000-03-01 so leap days fall at the end of the year.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

pub fn stamp(seconds: u64) -> String {
    let (year, month, day) = civil((seconds / 86400) as i64);
    let s = seconds % 86400;
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        s / 3600,
        s % 3600 / 60,
        s % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_first_of_january_1970() {
        assert_eq!(stamp(0), "19700101T000000Z");
    }

    #[test]
    fn leap_day_and_late_evening_round_trip() {
        assert_eq!(stamp(1_709_164_800 + 86_399), "20240229T235959Z");
    }

    #[test]
    fn known_instant_matches_date_utility() {
        assert_eq!(stamp(1_700_000_000), "20231114T221320Z");
    }
}
