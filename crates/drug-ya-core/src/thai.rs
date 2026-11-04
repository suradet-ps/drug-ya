//! Thai and Buddhist-era display formatting shared by the screen and any
//! future printed review note.

use chrono::{Datelike, NaiveDate, NaiveTime};

const THAI_MONTHS: [&str; 12] = [
    "มกราคม",
    "กุมภาพันธ์",
    "มีนาคม",
    "เมษายน",
    "พฤษภาคม",
    "มิถุนายน",
    "กรกฎาคม",
    "สิงหาคม",
    "กันยายน",
    "ตุลาคม",
    "พฤศจิกายน",
    "ธันวาคม",
];

const THAI_MONTHS_SHORT: [&str; 12] = [
    "ม.ค.",
    "ก.พ.",
    "มี.ค.",
    "เม.ย.",
    "พ.ค.",
    "มิ.ย.",
    "ก.ค.",
    "ส.ค.",
    "ก.ย.",
    "ต.ค.",
    "พ.ย.",
    "ธ.ค.",
];

/// Convert a Christian year to a Buddhist year.
pub fn be_year(year: i32) -> i32 {
    year + 543
}

/// Format a date as a full Thai/Buddhist-era date.
pub fn format_thai_date(date: NaiveDate) -> String {
    format!(
        "{} {} {}",
        date.day(),
        THAI_MONTHS[(date.month0() as usize).min(11)],
        be_year(date.year())
    )
}

/// Format a date as a compact Thai/Buddhist-era date.
pub fn format_thai_date_short(date: NaiveDate) -> String {
    format!(
        "{} {} {}",
        date.day(),
        THAI_MONTHS_SHORT[(date.month0() as usize).min(11)],
        be_year(date.year())
    )
}

/// Format a date and optional HOSxP time string.
pub fn format_thai_datetime(date: NaiveDate, time: Option<&str>) -> String {
    match time.and_then(format_time) {
        Some(t) => format!("{} เวลา {} น.", format_thai_date_short(date), t),
        None => format_thai_date_short(date),
    }
}

/// Normalize `HH:MM:SS` / `HH:MM` to `HH:MM`, ignoring invalid values.
pub fn format_time(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let mut parts = raw.split(':');
    let hour: u32 = parts.next()?.parse().ok()?;
    let minute: u32 = parts.next()?.parse().ok()?;
    if hour > 23 || minute > 59 {
        return None;
    }
    // A full `NaiveTime` parse also accepts values like "9:5"; normalize.
    NaiveTime::from_hms_opt(hour, minute, 0)?;
    Some(format!("{hour:02}:{minute:02}"))
}

/// Format a quantity with at most two decimals and no trailing zeros.
pub fn format_qty(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded - rounded.trunc()).abs() < f64::EPSILON {
        format!("{}", rounded as i64)
    } else {
        let s = format!("{rounded:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Format a number with a fixed number of decimals.
pub fn format_number(value: f64, decimals: usize) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    format!("{value:.decimals$}")
}

/// Whole years between `birth` and `today`, adjusted for the birth month/day.
pub fn age_years(birth: NaiveDate, today: NaiveDate) -> u32 {
    let mut age = today.year() - birth.year();
    if (today.month(), today.day()) < (birth.month(), birth.day()) {
        age -= 1;
    }
    age.max(0) as u32
}

/// Format an age with a Thai unit.
pub fn format_age(age: Option<u32>) -> String {
    match age {
        Some(age) => format!("{age} ปี"),
        None => "ไม่ทราบอายุ".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn buddhist_year_adds_543() {
        assert_eq!(be_year(2026), 2569);
    }

    #[test]
    fn full_and_short_dates_are_thai() {
        assert_eq!(format_thai_date(d(2026, 11, 4)), "4 พฤศจิกายน 2569");
        assert_eq!(format_thai_date_short(d(2026, 11, 4)), "4 พ.ย. 2569");
    }

    #[test]
    fn time_is_normalized_and_invalid_time_is_ignored() {
        assert_eq!(format_time("9:5:00").as_deref(), Some("09:05"));
        assert_eq!(format_time("25:00"), None);
        assert_eq!(format_time(""), None);
    }

    #[test]
    fn quantity_trims_trailing_zeroes() {
        assert_eq!(format_qty(10.0), "10");
        assert_eq!(format_qty(10.50), "10.5");
        assert_eq!(format_qty(1.234), "1.23");
    }

    #[test]
    fn age_respects_birthday_not_yet_reached() {
        let today = d(2026, 11, 4);
        assert_eq!(age_years(d(1950, 1, 1), today), 76);
        assert_eq!(age_years(d(1950, 12, 1), today), 75);
    }
}
