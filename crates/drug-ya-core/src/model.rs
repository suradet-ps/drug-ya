//! Serializable domain model shared by the Tauri backend and the Leptos
//! frontend. Nothing here touches a database or a filesystem.

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::rules::{Category, Severity};

/// Maximum number of patient search results.
pub const SEARCH_LIMIT: usize = 20;

/// Patient sex as recorded in HOSxP (`patient.sex`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Sex {
    /// HOSxP `1`, `M`, or `male`.
    Male,
    /// HOSxP `2`, `F`, or `female`.
    Female,
    /// Missing or unrecognized.
    #[default]
    Unknown,
}

impl Sex {
    /// Map a raw HOSxP value to the domain.
    pub fn from_hosxp(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "m" | "male" | "ช" | "ชาย" => Self::Male,
            "2" | "f" | "female" | "ญ" | "หญิง" => Self::Female,
            _ => Self::Unknown,
        }
    }

    /// Thai label.
    pub fn label_th(self) -> &'static str {
        match self {
            Self::Male => "ชาย",
            Self::Female => "หญิง",
            Self::Unknown => "ไม่ระบุ",
        }
    }
}

/// A patient identity row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Patient {
    /// Hospital number.
    pub hn: String,
    /// Thai national ID; may be empty on some HOSxP sites.
    pub cid: String,
    /// Full name with prefix.
    pub full_name_th: String,
    /// Birth date, Gregorian era.
    pub birth_date: Option<NaiveDate>,
    /// Sex.
    pub sex: Sex,
    /// Whole years at the review date.
    pub age_years: Option<u32>,
}

/// One OPD visit today.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayVisit {
    /// HOSxP visit number.
    pub vn: String,
    /// Visit date.
    pub date: NaiveDate,
    /// Visit time as `HH:MM`, when available.
    pub time: Option<String>,
    /// Queue number, when the site has the column.
    pub queue: Option<String>,
    /// Department code.
    pub department_code: String,
    /// Department name.
    pub department_name: String,
    /// Prescriber code.
    pub doctor_code: String,
    /// Prescriber name.
    pub doctor_name: String,
    /// Number of drug lines ordered in this visit.
    pub order_count: usize,
}

/// One allergy / ADR record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllergyRecord {
    /// Free-text agent, e.g. "Penicillin" or a trade name.
    pub agent: String,
    /// Reported symptom, free text.
    pub symptom: String,
    /// Reporter, when recorded.
    pub reporter: String,
    /// Report date (Gregorian).
    pub report_date: Option<NaiveDate>,
    /// Additional note.
    pub note: String,
    /// Structured drug code when the site populates it (empty on Sra Bos).
    pub icode: String,
    /// HOSxP seriousness id, preserved as text; site mapping may vary.
    pub seriousness_id: String,
    /// HOSxP `no_alert = 'Y'`; surfaced, never silently hidden.
    pub no_alert: bool,
    /// HOSxP `force_no_order = 'Y'`; treated as a hard-stop finding.
    pub force_no_order: bool,
}

/// One diagnosis row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosisRecord {
    /// ICD-10 code.
    pub icd10: String,
    /// Local condition label when the code is one the rule pack understands.
    pub label: Option<String>,
    /// Visit date.
    pub date: NaiveDate,
    /// HOSxP diagnosis type code (1 = principal, ...).
    pub diag_type: String,
}

/// The kind of laboratory value the engine understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LabKind {
    /// Serum creatinine, mg/dL.
    Creatinine,
    /// eGFR reported by the lab, mL/min/1.73 m².
    Egfr,
    /// Serum potassium, mmol/L.
    Potassium,
    /// INR.
    Inr,
    /// ALT / SGPT, U/L.
    Alt,
    /// AST / SGOT, U/L.
    Ast,
    /// HbA1c, %.
    Hba1c,
}

impl LabKind {
    /// Thai display label.
    pub fn label_th(self) -> &'static str {
        match self {
            Self::Creatinine => "Creatinine",
            Self::Egfr => "eGFR",
            Self::Potassium => "Potassium",
            Self::Inr => "INR",
            Self::Alt => "ALT",
            Self::Ast => "AST",
            Self::Hba1c => "HbA1c",
        }
    }

    /// Conventional unit for display.
    pub fn unit(self) -> &'static str {
        match self {
            Self::Creatinine => "mg/dL",
            Self::Egfr => "mL/min/1.73m²",
            Self::Potassium => "mmol/L",
            Self::Inr => "",
            Self::Alt | Self::Ast => "U/L",
            Self::Hba1c => "%",
        }
    }

    /// All kinds the connector tries to resolve.
    pub fn all() -> &'static [LabKind] {
        &[
            Self::Creatinine,
            Self::Egfr,
            Self::Potassium,
            Self::Inr,
            Self::Alt,
            Self::Ast,
            Self::Hba1c,
        ]
    }
}

/// A laboratory result with its original text preserved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabValue {
    /// Parsed numeric value.
    pub value: f64,
    /// Original result text (e.g. `<0.5`).
    pub text: String,
    /// Unit reported or conventionally assumed.
    pub unit: String,
    /// Date the result was ordered.
    pub measured_on: NaiveDate,
    /// Name as reported by the lab.
    pub item_name: String,
    /// Display flag from conservative thresholds.
    pub flag: LabFlag,
}

/// A display-only lab flag. It never changes a finding by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LabFlag {
    /// Below the usual reference range.
    Low,
    /// Within the usual range.
    Normal,
    /// Above the usual reference range.
    High,
    /// Not enough information to flag.
    Unknown,
}

/// Pregnancy status used by pregnancy rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PregnancyStatus {
    /// Confirmed or strongly suggested pregnant.
    Pregnant,
    /// Known not pregnant.
    NotPregnant,
    /// Not applicable (e.g. male patient).
    NotApplicable,
    /// No reliable data; pregnancy rules are skipped.
    #[default]
    Unknown,
}

impl PregnancyStatus {
    /// Thai label.
    pub fn label_th(self) -> &'static str {
        match self {
            Self::Pregnant => "ตั้งครรภ์",
            Self::NotPregnant => "ไม่ตั้งครรภ์",
            Self::NotApplicable => "ไม่เกี่ยวข้อง",
            Self::Unknown => "ไม่ทราบสถานะ",
        }
    }
}

/// Clinical context assembled from HOSxP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClinicalContext {
    /// Age at the review date.
    pub age_years: Option<u32>,
    /// Sex.
    pub sex: Sex,
    /// Latest weight in kg.
    pub weight_kg: Option<f64>,
    /// Latest height in cm.
    pub height_cm: Option<f64>,
    /// BMI, computed when weight and height exist.
    pub bmi: Option<f64>,
    /// Latest serum creatinine.
    pub creatinine: Option<LabValue>,
    /// eGFR (lab-reported or CKD-EPI 2021).
    pub egfr: Option<f64>,
    /// Where the eGFR came from.
    pub egfr_source: String,
    /// Cockcroft-Gault creatinine clearance.
    pub crcl: Option<f64>,
    /// Latest potassium.
    pub potassium: Option<LabValue>,
    /// Latest INR.
    pub inr: Option<LabValue>,
    /// Latest ALT.
    pub alt: Option<LabValue>,
    /// Latest AST.
    pub ast: Option<LabValue>,
    /// Latest HbA1c.
    pub hba1c: Option<LabValue>,
    /// Pregnancy status.
    pub pregnancy: PregnancyStatus,
    /// Where the pregnancy status came from.
    pub pregnancy_source: String,
    /// Recent diagnoses, newest first.
    pub diagnoses: Vec<DiagnosisRecord>,
    /// Latest past medical history free text.
    pub pmh: Option<String>,
    /// Allergy records.
    pub allergies: Vec<AllergyRecord>,
    /// True when the allergy query completed (even if it returned no rows).
    pub allergy_data_available: bool,
    /// True when the lab query completed (even if it returned no rows).
    pub lab_data_available: bool,
    /// True when the diagnosis query completed (even if it returned no rows).
    pub diagnosis_data_available: bool,
    /// True when the vitals query completed (even if it returned no rows).
    pub vitals_data_available: bool,
    /// A note about the medication scope (e.g. today's OPD orders only).
    pub medication_scope_note: String,
}

/// One drug order line from `opitemrece` joined to `drugitems`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLine {
    /// Stable line identity built from vn + item_no + icode + duplicate index.
    pub line_id: String,
    /// Visit number.
    pub vn: String,
    /// Drug code.
    pub icode: String,
    /// Drug name from `drugitems`.
    pub drug_name: String,
    /// Strength text.
    pub strength: String,
    /// Dispensing units.
    pub units: String,
    /// Complete directions for use joined from every HOSxP sig field.
    pub sig: String,
    /// Quantity.
    pub qty: f64,
    /// HOSxP item number.
    pub item_no: i64,
    /// Prescriber name.
    pub doctor_name: String,
    /// Order date, Gregorian.
    pub ordered_on: Option<NaiveDate>,
    /// Order time as `HH:MM`.
    pub ordered_at: Option<String>,
    /// Resolved generic key, when the dictionary knows the drug.
    pub drug_key: Option<String>,
    /// Thai generic label.
    pub generic_name: Option<String>,
    /// Therapeutic classes used by the rules.
    pub classes: Vec<String>,
    /// Whether the drug is on the high-alert list.
    pub high_alert: bool,
    /// Whether the dictionary resolved the drug.
    pub known: bool,
    /// Estimated milligrams per day, when the sig/strength allow it.
    pub daily_dose_mg: Option<f64>,
    /// Estimated days supply from qty and the parsed sig.
    pub days_supply: Option<f64>,
    /// Short reason when a days-supply estimate is unavailable.
    #[serde(default)]
    pub days_supply_note: String,
    /// How the dose was interpreted, or why it could not be.
    pub dose_note: String,
}

/// A finding produced by the rule engine or the allergy matcher.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// Unique id for UI acknowledgement within a review.
    pub id: String,
    /// Originating rule id (`allergy.direct` for a direct allergy match).
    pub rule_id: String,
    /// Category.
    pub category: Category,
    /// Severity after site overrides.
    pub severity: Severity,
    /// Short Thai title.
    pub title: String,
    /// What was found.
    pub detail: String,
    /// What the pharmacist should do.
    pub recommendation: String,
    /// Evidence / mechanism.
    pub evidence: String,
    /// Source citation.
    pub source: String,
    /// Patient-specific facts that triggered the rule.
    pub patient_factors: Vec<String>,
    /// Involved order line ids.
    pub drug_line_ids: Vec<String>,
    /// Involved drug names (display).
    pub drug_names: Vec<String>,
    /// Whether the UI should require explicit acknowledgement.
    pub requires_ack: bool,
    /// Extra data-quality note, if any.
    pub data_note: Option<String>,
}

impl Finding {
    /// Severity rank for sorting (lower is more urgent).
    pub fn rank(&self) -> u8 {
        self.severity.rank()
    }
}

/// Result of the deterministic review engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewAnalysis {
    /// Counts and headline.
    pub summary: ReviewSummary,
    /// Findings, most severe first.
    pub findings: Vec<Finding>,
    /// What was and was not checked.
    pub coverage: CoverageReport,
}

/// Counts and headline for the review.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSummary {
    /// Contraindicated / do-not-dispense findings.
    pub critical: usize,
    /// High-severity findings.
    pub high: usize,
    /// Moderate findings.
    pub moderate: usize,
    /// Low findings.
    pub low: usize,
    /// Informational / high-alert findings.
    pub info: usize,
    /// Critical + high + moderate + low.
    pub actionable: usize,
    /// A short Thai headline.
    pub headline: String,
    /// Overall visual state.
    pub risk_level: RiskLevel,
}

/// Overall review state used by the UI banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RiskLevel {
    /// No actionable findings.
    Clear,
    /// Low / informational only.
    Monitor,
    /// Moderate findings.
    Review,
    /// Critical or high findings.
    Urgent,
}

/// Coverage of one check category.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryCoverage {
    /// Category.
    pub category: Category,
    /// Checked / partial / skipped.
    pub status: crate::rules::CheckStatus,
    /// Number of enabled rules that ran.
    pub checked_rules: usize,
    /// Number of findings in this category.
    pub findings: usize,
    /// Short Thai note.
    pub note: String,
}

/// One missing input that limited the review.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingData {
    /// What is missing (Thai).
    pub label: String,
    /// Why it matters (Thai).
    pub detail: String,
    /// Categories affected.
    pub affects: Vec<Category>,
}

/// What the engine could and could not check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageReport {
    /// One row per category.
    pub categories: Vec<CategoryCoverage>,
    /// Missing data that limited the review.
    pub missing_data: Vec<MissingData>,
    /// Drug names the dictionary could not resolve.
    pub unresolved_drugs: Vec<String>,
    /// Number of orders with a known generic.
    pub known_drug_count: usize,
    /// Total number of orders.
    pub total_order_count: usize,
    /// Enabled rules that ran.
    pub checked_rule_count: usize,
    /// Rules that produced at least one finding.
    pub applied_rule_count: usize,
    /// Additional caveats (Thai).
    pub notes: Vec<String>,
}

/// Rule pack metadata sent with every review and shown in settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulePackInfo {
    /// Stable pack id.
    pub id: String,
    /// Human name.
    pub name: String,
    /// Version string.
    pub version: String,
    /// Effective date.
    pub effective_date: NaiveDate,
    /// Short description.
    pub description: String,
    /// Number of rules in the pack.
    pub rule_count: usize,
    /// Whether the current pack version has been signed off.
    pub signed_off: bool,
    /// Sign-off details, when present.
    pub signoff: Option<crate::rules::RulePackSignoff>,
}

/// One complete patient review, ready for the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewResult {
    /// Patient identity.
    pub patient: Patient,
    /// Review date (today).
    pub review_date: NaiveDate,
    /// Today's visits.
    pub visits: Vec<TodayVisit>,
    /// Today's drug orders across all visits.
    pub orders: Vec<OrderLine>,
    /// Clinical context.
    pub clinical: ClinicalContext,
    /// Summary counts.
    pub summary: ReviewSummary,
    /// Findings.
    pub findings: Vec<Finding>,
    /// Coverage report.
    pub coverage: CoverageReport,
    /// Data-source warnings (Thai).
    pub warnings: Vec<String>,
    /// Rule pack metadata.
    pub rule_pack: RulePackInfo,
    /// Review timestamp as a Thai display string.
    pub checked_at: String,
}

/// A multi-line raw data bundle used internally by the connector before the
/// engine runs. Not sent over IPC.
#[derive(Debug, Clone)]
pub struct ReviewDataset {
    /// Patient.
    pub patient: Patient,
    /// Visits today.
    pub visits: Vec<TodayVisit>,
    /// Orders today.
    pub orders: Vec<OrderLine>,
    /// Clinical context.
    pub clinical: ClinicalContext,
    /// Warnings from degraded queries.
    pub warnings: Vec<String>,
    /// Review date.
    pub review_date: NaiveDate,
}

/// Format an order date/time into a short Thai label.
pub fn order_time_label(order: &OrderLine) -> String {
    match (order.ordered_on, order.ordered_at.as_deref()) {
        (Some(date), Some(time)) => crate::thai::format_thai_datetime(date, Some(time)),
        (Some(date), None) => crate::thai::format_thai_date_short(date),
        _ => String::new(),
    }
}

/// Format a `NaiveTime` as HH:MM.
pub fn time_label(time: NaiveTime) -> String {
    time.format("%H:%M").to_string()
}
