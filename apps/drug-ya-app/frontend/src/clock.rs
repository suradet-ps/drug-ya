//! Browser clock helpers.
//!
//! `chrono::Local::now()` calls `std::time::SystemTime::now()`, which panics in
//! a wasm32 browser build. The UI only needs a display date/time, so read the
//! JavaScript `Date` directly and format it with the shared Thai helpers.

/// Today's date as a full Thai/Buddhist-era label.
pub fn today_thai() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let date = js_sys::Date::new_0();
        let year = date.get_full_year() as i32;
        let month = date.get_month() + 1;
        let day = date.get_date();
        if let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
            return drug_ya_core::thai::format_thai_date(date);
        }
    }
    "วันนี้".to_string()
}

/// Current local time as `DD/MM/BBBB HH:MM` (Buddhist era).
pub fn stamp_thai() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let date = js_sys::Date::new_0();
        format!(
            "{:02}/{:02}/{} {:02}:{:02}",
            date.get_date(),
            date.get_month() + 1,
            date.get_full_year() + 543,
            date.get_hours(),
            date.get_minutes()
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "วันนี้".to_string()
    }
}
