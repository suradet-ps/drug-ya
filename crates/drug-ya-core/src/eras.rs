//! Date era normalization.
//!
//! HOSxP sites may store dates in the Buddhist era (พ.ศ.), the Christian era
//! (ค.ศ.), or even mixed values. There is no site-wide era setting: each value
//! is normalized by its year. Years ≥ 2500 are treated as Buddhist era; no
//! plausible patient record predates CE 1957.
//!
//! Date filters sent to MySQL use the current Christian date; display and
//! clinical calculations normalize each returned value. If a site stores
//! `vstdate` in the Buddhist era, the site-specific query needs review - the
//! verification package calls this out.

use chrono::{Datelike, NaiveDate};

/// First year that is unambiguously Buddhist era for patient records.
pub const BUDDHIST_ERA_YEAR_THRESHOLD: i32 = 2500;

/// Whether a stored year is Buddhist era (พ.ศ.).
pub fn is_buddhist_era_year(year: i32) -> bool {
    year >= BUDDHIST_ERA_YEAR_THRESHOLD
}

/// Convert a Buddhist-era date to Christian era.
pub fn buddhist_to_christian(date: NaiveDate) -> NaiveDate {
    let ce_year = date.year() - 543;
    date.with_year(ce_year)
        .expect("invariant: Buddhist-era Feb 29 only exists for leap CE years")
}

/// Normalize one HOSxP date to Christian era.
pub fn normalize_date(date: NaiveDate) -> NaiveDate {
    if is_buddhist_era_year(date.year()) {
        buddhist_to_christian(date)
    } else {
        date
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_both_eras() {
        assert_eq!(
            normalize_date(NaiveDate::from_ymd_opt(2567, 5, 10).unwrap()),
            NaiveDate::from_ymd_opt(2024, 5, 10).unwrap()
        );
        assert_eq!(
            normalize_date(NaiveDate::from_ymd_opt(2024, 5, 10).unwrap()),
            NaiveDate::from_ymd_opt(2024, 5, 10).unwrap()
        );
    }

    #[test]
    fn threshold_is_2500() {
        assert!(is_buddhist_era_year(2500));
        assert!(!is_buddhist_era_year(2499));
    }
}
