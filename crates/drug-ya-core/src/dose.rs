//! Conservative dose and days-supply estimation from HOSxP's free-text sig.
//!
//! HOSxP stores directions as a lookup text (`drugusage.name1..3`) plus a
//! free-text `sp_use`. The shapes vary by hospital. This module only returns
//! a milligram-per-day value when the strength, unit, and sig are all
//! interpretable; otherwise it returns `None` and a human-readable reason so
//! the review can say "ตรวจขนาดยาไม่ได้" instead of quietly guessing.

use serde::{Deserialize, Serialize};

/// The result of interpreting one order's strength + sig.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoseEstimate {
    /// Dosage units per day (tablets, capsules, or mL).
    pub units_per_day: Option<f64>,
    /// Milligrams of active drug per day, when this can be derived safely.
    pub mg_per_day: Option<f64>,
    /// Days supply derived from qty / units-per-day.
    pub days_supply: Option<f64>,
    /// Short reason when a days-supply estimate is not available.
    pub days_supply_note: String,
    /// Thai explanation for the UI.
    pub note: String,
}

/// Interpret one order.
pub fn estimate_dose(strength: &str, units: &str, sig: &str, qty: f64) -> DoseEstimate {
    let strength_info = parse_strength(strength);
    let sig_info = parse_sig(sig);
    let unit_kind = classify_units(units, &strength_info);

    // Liquid sigs are expressed in spoons or mL. A spoon must be converted to
    // its mL volume before dividing the dispensed quantity; "1x3" for a liquid
    // remains ambiguous and gets no days estimate.
    let per_day_quantity = match unit_kind {
        UnitKind::Solid | UnitKind::NonSolid => sig_info.dose_units,
        UnitKind::Liquid => sig_info.volume_ml_per_dose,
    };
    let units_per_day = per_day_quantity
        .zip(sig_info.times_per_day)
        .map(|(dose, freq)| dose * freq)
        .filter(|v| v.is_finite() && *v > 0.0);

    let days_supply = units_per_day
        .filter(|v| *v > 0.0)
        .map(|per_day| qty / per_day)
        .filter(|v| v.is_finite() && *v > 0.0);

    let mg_per_day = if let Some(per_day) = units_per_day {
        match unit_kind {
            UnitKind::Solid => strength_info.mg_per_unit.map(|mg| per_day * mg),
            UnitKind::Liquid => strength_info.mg_per_ml.map(|mg_per_ml| per_day * mg_per_ml),
            UnitKind::NonSolid => None,
        }
    } else {
        None
    };

    let days_supply_note = if days_supply.is_some() {
        String::new()
    } else if sig.trim().is_empty() {
        "ไม่มีวิธีใช้".to_string()
    } else if sig_info.prn && sig_info.times_per_day.is_none() {
        "ตามอาการ".to_string()
    } else if sig_info.times_per_day.is_none() {
        "ไม่ระบุความถี่".to_string()
    } else if matches!(unit_kind, UnitKind::Liquid) && sig_info.volume_ml_per_dose.is_none() {
        "ไม่ระบุปริมาตร".to_string()
    } else {
        "อ่านไม่ชัด".to_string()
    };

    let note = if sig.trim().is_empty() {
        "ไม่มีข้อความวิธีใช้ (sig) ใน HOSxP".to_string()
    } else if sig_info.prn && sig_info.times_per_day.is_none() {
        "ใช้ตามอาการ; ไม่ระบุความถี่ในวิธีใช้".to_string()
    } else if sig_info.times_per_day.is_none() {
        format!("ไม่ระบุความถี่ในวิธีใช้: {}", sig.trim())
    } else if matches!(unit_kind, UnitKind::Liquid) && sig_info.volume_ml_per_dose.is_none() {
        "ยาน้ำ: ไม่ระบุปริมาตรต่อครั้ง; ประเมินจำนวนวันไม่ได้".to_string()
    } else if units_per_day.is_none() {
        format!("อ่านวิธีใช้ไม่ชัด: {}", sig.trim())
    } else if matches!(unit_kind, UnitKind::Liquid) && mg_per_day.is_none() {
        "ยาน้ำ/ยาฉีด: ไม่ประเมิน mg/วัน จาก sig นี้".to_string()
    } else if matches!(unit_kind, UnitKind::NonSolid) {
        "รูปแบบยาไม่ใช่ยารับประทานชนิดเม็ด/แคปซูล: ไม่ประเมิน mg/วัน".to_string()
    } else if mg_per_day.is_none() {
        "ไม่พบความแรงที่เป็น mg ในบัญชียา".to_string()
    } else {
        let mut text = format!(
            "{} = {} หน่วย/วัน",
            sig.trim(),
            crate::thai::format_qty(units_per_day.unwrap_or(0.0))
        );
        if sig_info.prn {
            text.push_str(" (ใช้ตามอาการ)");
        }
        text
    };

    DoseEstimate {
        units_per_day,
        mg_per_day,
        days_supply,
        days_supply_note,
        note,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnitKind {
    Solid,
    Liquid,
    NonSolid,
}

fn classify_units(units: &str, strength: &StrengthInfo) -> UnitKind {
    if strength.mg_per_ml.is_some() {
        return UnitKind::Liquid;
    }
    let u = units.to_ascii_lowercase();
    let non_solid = [
        "inj",
        "amp",
        "vial",
        "syringe",
        "drop",
        "spray",
        "cream",
        "oint",
        "gel",
        "patch",
        "supp",
        "susp",
        "solution",
        "syrup",
        "elixir",
        "sachet",
        "ซอง",
        "หลอด",
        "ขวด",
        "หยด",
        "ครีม",
        "ขี้ผึ้ง",
        "เจล",
        "แผ่น",
    ];
    if non_solid.iter().any(|needle| u.contains(needle)) {
        return UnitKind::NonSolid;
    }
    let liquid = ["ml", "cc", "ซีซี", "มิลลิลิตร"];
    if liquid.iter().any(|needle| u.contains(needle)) {
        return UnitKind::Liquid;
    }
    UnitKind::Solid
}

#[derive(Debug, Clone, Copy, Default)]
struct StrengthInfo {
    mg_per_unit: Option<f64>,
    mg_per_ml: Option<f64>,
}

fn parse_strength(raw: &str) -> StrengthInfo {
    let text = raw.to_ascii_lowercase();
    let Some((number, number_end)) = first_number(&text) else {
        return StrengthInfo::default();
    };
    let after = text[number_end..].trim_start();
    let (factor, unit_len) = if after.starts_with("mcg") || after.starts_with("µg") {
        (0.001, 3)
    } else if after.starts_with("mg") {
        (1.0, 2)
    } else if after.starts_with("g") && !after.starts_with("gr") {
        (1000.0, 1)
    } else {
        return StrengthInfo::default();
    };
    let mg = number * factor;
    let rest = after[unit_len..].trim_start();
    if let Some(rest) = rest.strip_prefix('/') {
        let rest = rest.trim_start();
        if rest.starts_with("ml") {
            return StrengthInfo {
                mg_per_unit: Some(mg),
                mg_per_ml: Some(mg),
            };
        }
        if let Some((volume, end)) = first_number(rest) {
            let unit = rest[end..].trim_start();
            if unit.starts_with("ml") && volume > 0.0 {
                return StrengthInfo {
                    mg_per_unit: Some(mg),
                    mg_per_ml: Some(mg / volume),
                };
            }
        }
    }
    StrengthInfo {
        mg_per_unit: Some(mg),
        mg_per_ml: None,
    }
}

/// Return the first decimal number and the byte index just after it.
fn first_number(text: &str) -> Option<(f64, usize)> {
    let bytes = text.as_bytes();
    let mut start = None;
    for (i, b) in bytes.iter().enumerate() {
        if b.is_ascii_digit() || (*b == b'.' && start.is_none()) {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start {
            if i > s {
                let slice = &text[s..i];
                return slice.parse::<f64>().ok().map(|v| (v, i));
            }
            start = None;
        }
    }
    let s = start?;
    text[s..].parse::<f64>().ok().map(|v| (v, text.len()))
}

#[derive(Debug, Clone, Default)]
struct SigInfo {
    dose_units: Option<f64>,
    times_per_day: Option<f64>,
    volume_ml_per_dose: Option<f64>,
    prn: bool,
}

fn parse_sig(raw: &str) -> SigInfo {
    let text = raw
        .trim()
        .to_ascii_lowercase()
        .replace('×', "x")
        .replace(['–', '—'], "-");
    if text.is_empty() {
        return SigInfo::default();
    }
    let mut info = SigInfo {
        prn: contains_any(
            &text,
            &[
                "prn",
                "p.r.n",
                "sos",
                "ตามอาการ",
                "เมื่อมีอาการ",
                "เมื่อจำเป็น",
                "นอนไม่หลับ",
                "เวลานอนไม่หลับ",
                "เมื่อนอนไม่หลับ",
                "ถ้านอนไม่หลับ",
            ],
        ),
        ..SigInfo::default()
    };

    // 1x3 / 2 x 4 / 1x3x7
    if let Some((dose, freq)) = x_pattern(&text) {
        info.dose_units = Some(dose);
        info.times_per_day = Some(freq);
    }

    // 1-0-1 / 1+0+1 / 1-0-0-1
    if info.dose_units.is_none()
        && let Some((dose, freq)) = dash_pattern(&text)
    {
        info.dose_units = Some(dose);
        info.times_per_day = Some(freq);
    }

    // q8h / q6h / q12h
    if info.times_per_day.is_none()
        && let Some(hours) = q_hours_pattern(&text)
        && hours > 0.0
    {
        info.times_per_day = Some((24.0 / hours).min(24.0));
    }

    // bid / tid / qid / od / qd / hs
    if info.times_per_day.is_none() {
        info.times_per_day = word_frequency(&text);
    }

    // "วันละ 3 ครั้ง" / "วันละ 2 เวลา" / "ทุก 8 ชั่วโมง"
    if info.times_per_day.is_none() {
        info.times_per_day = thai_frequency(&text);
    }

    // "ก่อนนอน" is a once-nightly instruction unless the sig also says the
    // medicine is used only when needed. A PRN bedtime sig is left unestimated.
    if info.times_per_day.is_none()
        && !info.prn
        && contains_any(&text, &["ก่อนนอน", "เข้านอน", "เวลานอน"])
    {
        info.times_per_day = Some(1.0);
    }

    // "ครั้งละ 2 เม็ด" / "ครั้งละ ครึ่ง เม็ด" / "ครั้งละ 1/2 เม็ด"
    if info.dose_units.is_none() {
        info.dose_units = thai_dose_pattern(&text).or_else(|| dosage_unit_pattern(&text));
    }

    // If the frequency is explicit but the dose is not, one dosage unit is a
    // reasonable default only when the sig names a solid unit.
    if info.dose_units.is_none()
        && info.times_per_day.is_some()
        && contains_any(&text, &["tab", "cap", "tablet", "capsule", "เม็ด", "แคปซูล"])
    {
        info.dose_units = Some(1.0);
    }

    // Volume per dose: "5 ml", "1 ช้อนชา" (5 mL), "1 ช้อนโต๊ะ" (15 mL).
    if let Some(ml) = volume_per_dose(&text) {
        info.volume_ml_per_dose = Some(ml);
    }

    info
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn x_pattern(text: &str) -> Option<(f64, f64)> {
    let bytes = text.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b != b'x' {
            continue;
        }
        let left = text[..i].trim_end();
        let right = text[i + 1..].trim_start();
        let (dose, _) = last_number(left)?;
        let (freq, _) = first_number(right)?;
        if dose > 0.0 && freq > 0.0 && freq <= 24.0 {
            return Some((dose, freq));
        }
    }
    None
}

fn dash_pattern(text: &str) -> Option<(f64, f64)> {
    if !text.contains('-') && !text.contains('+') {
        return None;
    }
    let normalized = text.replace('+', "-");
    let parts: Vec<&str> = normalized
        .split(|c: char| c == '-' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();
    let mut numbers = Vec::new();
    for part in parts {
        if let Ok(value) = part.parse::<f64>() {
            numbers.push(value);
        } else if part.contains('/')
            && let Some(value) = half_fraction(part)
        {
            numbers.push(value);
        }
        if numbers.len() == 4 {
            break;
        }
    }
    if numbers.len() >= 3 {
        let total: f64 = numbers.iter().take(4).sum();
        if total > 0.0 {
            return Some((total, 1.0));
        }
    }
    None
}

fn half_fraction(text: &str) -> Option<f64> {
    let (a, b) = text.split_once('/')?;
    let a: f64 = a.trim().parse().ok()?;
    let b: f64 = b.trim().parse().ok()?;
    (b > 0.0).then_some(a / b)
}

fn q_hours_pattern(text: &str) -> Option<f64> {
    let idx = text.find('q')?;
    let rest = &text[idx + 1..];
    let (value, end) = first_number(rest)?;
    if rest[end..].trim_start().starts_with('h') {
        Some(value)
    } else {
        None
    }
}

fn word_frequency(text: &str) -> Option<f64> {
    let words: Vec<&str> = text
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|s| !s.is_empty())
        .collect();
    for word in words {
        match word {
            "od" | "qd" | "daily" | "hs" | "qhs" | "qam" | "qpm" => return Some(1.0),
            "bid" => return Some(2.0),
            "tid" => return Some(3.0),
            "qid" => return Some(4.0),
            _ => {}
        }
    }
    None
}

fn thai_frequency(text: &str) -> Option<f64> {
    if let Some(idx) = text.find("วันละ") {
        let rest = &text[idx..];
        if let Some((value, _)) = first_number(rest) {
            return Some(value);
        }
    }
    if let Some(idx) = text.find("ทุก") {
        let rest = &text[idx..];
        if let Some((value, _)) = first_number(rest)
            && rest.contains("ชั่วโมง")
            && value > 0.0
        {
            return Some((24.0 / value).min(24.0));
        }
    }
    None
}

fn thai_dose_pattern(text: &str) -> Option<f64> {
    let idx = text.find("ครั้งละ")?;
    let rest = &text[idx + "ครั้งละ".len()..];
    let dose_segment = rest.find("วันละ").map(|end| &rest[..end]).unwrap_or(rest);
    quantity_prefix(dose_segment)
}

fn quantity_prefix(text: &str) -> Option<f64> {
    let text =
        text.trim_start_matches(|c: char| c.is_whitespace() || matches!(c, '·' | ':' | '-' | '•'));
    for (word, value) in [
        ("ครึ่ง", 0.5),
        ("หนึ่ง", 1.0),
        ("สอง", 2.0),
        ("สาม", 3.0),
        ("สี่", 4.0),
        ("ห้า", 5.0),
    ] {
        if text.starts_with(word) {
            return Some(value);
        }
    }
    if text.starts_with('½') {
        return Some(0.5);
    }
    if text.starts_with('¼') {
        return Some(0.25);
    }

    let (value, end) = first_number(text)?;
    let rest = text[end..].trim_start();
    if let Some(rest) = rest.strip_prefix('/')
        && let Some((denominator, _)) = first_number(rest.trim_start())
        && denominator > 0.0
    {
        return Some(value / denominator);
    }
    (value > 0.0).then_some(value)
}

fn dosage_unit_pattern(text: &str) -> Option<f64> {
    let idx = text.find("tab").or_else(|| text.find("cap"))?;
    let before = &text[..idx];
    let (value, _) = last_number(before)?;
    (value > 0.0).then_some(value)
}

fn volume_per_dose(text: &str) -> Option<f64> {
    if let Some(idx) = text.find("ml") {
        let before = &text[..idx];
        let (value, _) = last_number(before)?;
        return (value > 0.0).then_some(value);
    }
    if text.contains("ช้อนชา") {
        let (value, _) = first_number(text)?;
        return Some(value * 5.0);
    }
    if text.contains("ช้อนโต๊ะ") {
        let (value, _) = first_number(text)?;
        return Some(value * 15.0);
    }
    if let Some(idx) = text.find("cc") {
        let before = &text[..idx];
        let (value, _) = last_number(before)?;
        return (value > 0.0).then_some(value);
    }
    None
}

fn last_number(text: &str) -> Option<(f64, usize)> {
    let bytes = text.as_bytes();
    let mut end = None;
    for (i, b) in bytes.iter().enumerate().rev() {
        if b.is_ascii_digit() || *b == b'.' || *b == b'/' {
            if end.is_none() {
                end = Some(i + 1);
            }
        } else if let Some(e) = end {
            let mut start = i + 1;
            while start < e && !bytes[start].is_ascii_digit() {
                start += 1;
            }
            let slice = &text[start..e];
            if let Some(value) = half_fraction(slice) {
                return Some((value, e));
            }
            return slice.trim().parse::<f64>().ok().map(|v| (v, e));
        }
    }
    let e = end?;
    let slice = &text[..e];
    if let Some(value) = half_fraction(slice) {
        return Some((value, e));
    }
    slice.trim().parse::<f64>().ok().map(|v| (v, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_sig_shapes() {
        let a = estimate_dose("500 mg", "Tab", "1x3", 30.0);
        assert_eq!(a.units_per_day, Some(3.0));
        assert_eq!(a.mg_per_day, Some(1500.0));
        assert!((a.days_supply.unwrap() - 10.0).abs() < 0.01);

        let b = estimate_dose("500 mg", "Tab", "1-0-1", 20.0);
        assert_eq!(b.units_per_day, Some(2.0));
        assert_eq!(b.mg_per_day, Some(1000.0));

        let c = estimate_dose("250 mg", "Cap", "1 cap q8h", 30.0);
        assert_eq!(c.units_per_day, Some(3.0));
        assert_eq!(c.mg_per_day, Some(750.0));
    }

    #[test]
    fn parses_thai_frequency() {
        let d = estimate_dose("500 mg", "เม็ด", "1 เม็ด วันละ 3 ครั้ง", 30.0);
        assert_eq!(d.units_per_day, Some(3.0));
        assert_eq!(d.mg_per_day, Some(1500.0));

        let two_times = estimate_dose(
            "500 mg",
            "เม็ด",
            "ครั้งละ 1 เม็ด · วันละ 2 เวลา หลังอาหาร · เช้า-เย็น",
            30.0,
        );
        assert_eq!(two_times.units_per_day, Some(2.0));
        assert!((two_times.days_supply.unwrap() - 15.0).abs() < 0.01);
    }

    #[test]
    fn parses_thai_dose_from_khrang_la() {
        let one = estimate_dose(
            "500 mg",
            "เม็ด",
            "รับประทาน ครั้งละ 1 เม็ด · วันละ 1 ครั้ง · ก่อนนอน",
            15.0,
        );
        assert_eq!(one.units_per_day, Some(1.0));
        assert!((one.days_supply.unwrap() - 15.0).abs() < 0.01);
        assert!(one.days_supply_note.is_empty());

        let two = estimate_dose(
            "500 mg",
            "เม็ด",
            "รับประทาน ครั้งละ 2 เม็ด · วันละ 1 ครั้ง หลังอาหาร · เช้า",
            30.0,
        );
        assert_eq!(two.units_per_day, Some(2.0));
        assert!((two.days_supply.unwrap() - 15.0).abs() < 0.01);
    }

    #[test]
    fn parses_half_tablet_and_fractional_tablet() {
        let half_word = estimate_dose(
            "500 mg",
            "เม็ด",
            "รับประทานครั้งละ ครึ่ง เม็ด · วันละ 2 ครั้ง หลังอาหาร · เช้า - ก่อนนอน",
            30.0,
        );
        assert_eq!(half_word.units_per_day, Some(1.0));
        assert!((half_word.days_supply.unwrap() - 30.0).abs() < 0.01);

        let half_fraction =
            estimate_dose("500 mg", "เม็ด", "รับประทานครั้งละ 1/2 เม็ด · วันละ 2 ครั้ง", 30.0);
        assert_eq!(half_fraction.units_per_day, Some(1.0));
        assert!((half_fraction.days_supply.unwrap() - 30.0).abs() < 0.01);
    }

    #[test]
    fn scheduled_bedtime_counts_as_once_daily() {
        let d = estimate_dose("500 mg", "เม็ด", "รับประทานครั้งละ 1 เม็ด · ก่อนนอน", 30.0);
        assert_eq!(d.units_per_day, Some(1.0));
        assert!((d.days_supply.unwrap() - 30.0).abs() < 0.01);
    }

    #[test]
    fn prn_bedtime_is_not_given_a_days_estimate() {
        let d = estimate_dose(
            "500 mg",
            "เม็ด",
            "รับประทานครั้งละ 1 เม็ด · ก่อนนอน · เวลานอนไม่หลับ",
            30.0,
        );
        assert_eq!(d.units_per_day, None);
        assert_eq!(d.days_supply, None);
        assert_eq!(d.days_supply_note, "ตามอาการ");
        assert!(d.note.contains("ใช้ตามอาการ"));
    }

    #[test]
    fn liquid_with_explicit_volume_is_convertible() {
        let e = estimate_dose("250 mg/5 ml", "mL", "5 mL x 3", 150.0);
        assert_eq!(e.units_per_day, Some(15.0));
        assert_eq!(e.mg_per_day, Some(750.0));
        assert!((e.days_supply.unwrap() - 10.0).abs() < 0.01);

        let spoon = estimate_dose("250 mg/5 ml", "mL", "1 ช้อนชา x 3", 150.0);
        assert_eq!(spoon.units_per_day, Some(15.0));
        assert_eq!(spoon.mg_per_day, Some(750.0));
        assert!((spoon.days_supply.unwrap() - 10.0).abs() < 0.01);
    }

    #[test]
    fn ambiguous_liquid_is_not_guessed() {
        let e = estimate_dose("250 mg/5 ml", "mL", "1x3", 100.0);
        assert_eq!(e.units_per_day, None);
        assert_eq!(e.days_supply, None);
        assert_eq!(e.days_supply_note, "ไม่ระบุปริมาตร");
        assert!(e.note.contains("ยาน้ำ"));
    }

    #[test]
    fn injection_is_not_dosed_as_an_oral_tablet() {
        let e = estimate_dose("1 g", "Vial", "1x1", 1.0);
        assert_eq!(e.mg_per_day, None);
        assert!(e.note.contains("ไม่ใช่ยารับประทาน"));
    }

    #[test]
    fn unreadable_sig_is_reported_not_guessed() {
        let e = estimate_dose("500 mg", "Tab", "ตามแพทย์สั่ง", 10.0);
        assert_eq!(e.units_per_day, None);
        assert!(e.note.contains("ไม่ระบุความถี่"));
    }
}
