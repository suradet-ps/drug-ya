//! Renal function estimates.
//!
//! Two numbers are shown to the pharmacist because they answer different
//! questions:
//! - **CKD-EPI 2021** is the modern eGFR equation (no race coefficient) and
//!   is used for CKD staging.
//! - **Cockcroft-Gault** is still the equation many drug labels use for
//!   renal dose adjustment; it needs a current body weight.
//!
//! Neither replaces clinical judgment. The app shows the equation and the
//! input data so the pharmacist can see why a number is what it is.

use crate::model::Sex;

/// CKD-EPI 2021 creatinine equation (mL/min/1.73 m²).
///
/// `scr_mg_dl` must be positive; `age` is whole years.
pub fn ckd_epi_2021(scr_mg_dl: f64, age: u32, sex: Sex) -> Option<f64> {
    if !(scr_mg_dl.is_finite() && scr_mg_dl > 0.0) || age > 130 {
        return None;
    }
    let female = matches!(sex, Sex::Female);
    let k = if female { 0.7 } else { 0.9 };
    let a = if female { -0.241 } else { -0.302 };
    let ratio = scr_mg_dl / k;
    let value = 142.0
        * ratio.min(1.0).powf(a)
        * ratio.max(1.0).powf(-1.200)
        * 0.9938_f64.powi(age as i32)
        * if female { 1.012 } else { 1.0 };
    (value.is_finite() && value > 0.0).then_some(value)
}

/// Cockcroft-Gault creatinine clearance (mL/min).
///
/// Uses actual body weight, as most drug labels do. Returns `None` when the
/// weight is missing or invalid.
pub fn cockcroft_gault(scr_mg_dl: f64, age: u32, weight_kg: Option<f64>, sex: Sex) -> Option<f64> {
    let weight = weight_kg.filter(|w| w.is_finite() && *w > 0.0)?;
    if !(scr_mg_dl.is_finite() && scr_mg_dl > 0.0) || age > 130 {
        return None;
    }
    let mut value = ((140.0 - age as f64) * weight) / (72.0 * scr_mg_dl);
    if matches!(sex, Sex::Female) {
        value *= 0.85;
    }
    (value.is_finite() && value > 0.0).then_some(value)
}

/// CKD stage label from an eGFR value.
pub fn ckd_stage(egfr: f64) -> &'static str {
    if egfr >= 90.0 {
        "G1"
    } else if egfr >= 60.0 {
        "G2"
    } else if egfr >= 45.0 {
        "G3a"
    } else if egfr >= 30.0 {
        "G3b"
    } else if egfr >= 15.0 {
        "G4"
    } else {
        "G5"
    }
}

/// A short Thai interpretation of an eGFR value.
pub fn ckd_stage_thai(egfr: f64) -> &'static str {
    match ckd_stage(egfr) {
        "G1" => "ปกติหรือสูง (G1)",
        "G2" => "ลดลงเล็กน้อย (G2)",
        "G3a" => "ลดลงเล็กน้อยถึงปานกลาง (G3a)",
        "G3b" => "ลดลงปานกลางถึงมาก (G3b)",
        "G4" => "ลดลงมาก (G4)",
        _ => "ไตวายระยะสุดท้าย (G5)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ckd_epi_matches_a_reference_value() {
        // 50-year-old male, SCr 1.0 mg/dL -> about 92 mL/min/1.73 m²
        // with the 2021 creatinine equation (no race coefficient).
        let value = ckd_epi_2021(1.0, 50, Sex::Male).unwrap();
        assert!((value - 91.7).abs() < 1.0, "{value}");
    }

    #[test]
    fn ckd_epi_female_is_slightly_lower_than_male_at_same_inputs() {
        let male = ckd_epi_2021(1.0, 50, Sex::Male).unwrap();
        let female = ckd_epi_2021(1.0, 50, Sex::Female).unwrap();
        assert!(female < male);
    }

    #[test]
    fn cockcroft_gault_requires_weight() {
        assert_eq!(cockcroft_gault(1.0, 50, None, Sex::Male), None);
        let value = cockcroft_gault(1.0, 50, Some(70.0), Sex::Male).unwrap();
        assert!((value - 87.5).abs() < 0.1, "{value}");
    }

    #[test]
    fn female_cockcroft_gault_applies_the_085_factor() {
        let male = cockcroft_gault(1.0, 50, Some(70.0), Sex::Male).unwrap();
        let female = cockcroft_gault(1.0, 50, Some(70.0), Sex::Female).unwrap();
        assert!((female - male * 0.85).abs() < 0.01);
    }

    #[test]
    fn ckd_stages_are_ordered() {
        assert_eq!(ckd_stage(95.0), "G1");
        assert_eq!(ckd_stage(50.0), "G3a");
        assert_eq!(ckd_stage(10.0), "G5");
    }
}
