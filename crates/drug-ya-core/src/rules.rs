//! The configurable clinical rule pack.
//!
//! Every rule is explicit, named, sourced, and can be turned off or adjusted
//! by the site pharmacist. The bundled pack is a **starter**: it is written
//! to be conservative and explainable, not to be a complete drug knowledge
//! base. `docs/CLINICAL-RULES.md` records the review/sign-off workflow.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::model::{LabKind, RulePackInfo};

/// Severity shown to the pharmacist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// Contraindicated / do not dispense until the prescriber is contacted.
    Critical,
    /// Avoid or intervene; high risk of harm.
    High,
    /// Monitor, adjust, or confirm.
    Moderate,
    /// Awareness / low-risk issue.
    Low,
    /// High-alert checklist / context, not a problem by itself.
    Info,
}

impl Severity {
    /// Sort rank (lower is more urgent).
    pub fn rank(self) -> u8 {
        match self {
            Self::Critical => 0,
            Self::High => 1,
            Self::Moderate => 2,
            Self::Low => 3,
            Self::Info => 4,
        }
    }

    /// Thai label.
    pub fn label_th(self) -> &'static str {
        match self {
            Self::Critical => "วิกฤต",
            Self::High => "สูง",
            Self::Moderate => "ปานกลาง",
            Self::Low => "ต่ำ",
            Self::Info => "ข้อมูล",
        }
    }

    /// English code for CSS classes.
    pub fn code(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Moderate => "moderate",
            Self::Low => "low",
            Self::Info => "info",
        }
    }

    /// Whether the finding should require explicit acknowledgement.
    pub fn requires_ack(self) -> bool {
        matches!(self, Self::Critical | Self::High)
    }
}

/// Review categories, also used by the coverage report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    /// Allergy / ADR cross-check.
    Allergy,
    /// Drug-drug interaction.
    Interaction,
    /// Duplicate / therapeutic duplication.
    DuplicateTherapy,
    /// Renal dose / avoid.
    Renal,
    /// Maximum daily dose.
    Dose,
    /// Older-adult appropriateness.
    Geriatric,
    /// Drug-disease interaction.
    Disease,
    /// Drug-lab interaction.
    Lab,
    /// Pregnancy.
    Pregnancy,
    /// Pediatric.
    Pediatric,
    /// High-alert checklist.
    HighAlert,
}

impl Category {
    /// Thai label.
    pub fn label_th(self) -> &'static str {
        match self {
            Self::Allergy => "แพ้ยา",
            Self::Interaction => "ปฏิกิริยาระหว่างยา",
            Self::DuplicateTherapy => "ยาซ้ำ / กลุ่มเดียวกัน",
            Self::Renal => "การทำงานของไต",
            Self::Dose => "ขนาดยา",
            Self::Geriatric => "ผู้สูงอายุ",
            Self::Disease => "โรคประจำตัว",
            Self::Lab => "ผลตรวจทางห้องปฏิบัติการ",
            Self::Pregnancy => "การตั้งครรภ์",
            Self::Pediatric => "เด็ก",
            Self::HighAlert => "ยาที่ต้องตรวจสอบซ้ำ",
        }
    }

    /// A stable short code for CSS.
    pub fn code(self) -> &'static str {
        match self {
            Self::Allergy => "allergy",
            Self::Interaction => "interaction",
            Self::DuplicateTherapy => "duplicate",
            Self::Renal => "renal",
            Self::Dose => "dose",
            Self::Geriatric => "geriatric",
            Self::Disease => "disease",
            Self::Lab => "lab",
            Self::Pregnancy => "pregnancy",
            Self::Pediatric => "pediatric",
            Self::HighAlert => "highAlert",
        }
    }

    /// All categories in display order.
    pub fn all() -> &'static [Category] {
        &[
            Self::Allergy,
            Self::Interaction,
            Self::DuplicateTherapy,
            Self::Renal,
            Self::Dose,
            Self::Geriatric,
            Self::Disease,
            Self::Lab,
            Self::Pregnancy,
            Self::Pediatric,
            Self::HighAlert,
        ]
    }
}

/// Coverage state of one category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    /// All prerequisites were available.
    Checked,
    /// Some rules ran, but missing data or unknown drugs limited coverage.
    Partial,
    /// No meaningful check could run.
    Skipped,
}

impl CheckStatus {
    /// Thai label.
    pub fn label_th(self) -> &'static str {
        match self {
            Self::Checked => "ตรวจแล้ว",
            Self::Partial => "ตรวจได้บางส่วน",
            Self::Skipped => "ยังไม่ได้ตรวจ",
        }
    }
}

/// Which renal metric a rule uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum RenalMetric {
    /// Use Cockcroft-Gault when weight is available, else eGFR.
    #[default]
    Auto,
    /// Cockcroft-Gault CrCl.
    CockcroftGault,
    /// CKD-EPI 2021 eGFR.
    CkdEpi,
}

impl RenalMetric {
    /// Thai label.
    pub fn label_th(self) -> &'static str {
        match self {
            Self::Auto => "อัตโนมัติ (CrCl ถ้ามีน้ำหนัก)",
            Self::CockcroftGault => "Cockcroft-Gault (CrCl)",
            Self::CkdEpi => "CKD-EPI 2021 (eGFR)",
        }
    }
}

/// Finding sort order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum FindingSort {
    /// Most severe first.
    #[default]
    Severity,
    /// Category order.
    Category,
}

/// A comparison operator for lab thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompareOp {
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Ge,
    /// Less than.
    Lt,
    /// Less than or equal.
    Le,
}

impl CompareOp {
    /// Evaluate `value op threshold`.
    pub fn evaluate(self, value: f64, threshold: f64) -> bool {
        match self {
            Self::Gt => value > threshold,
            Self::Ge => value >= threshold,
            Self::Lt => value < threshold,
            Self::Le => value <= threshold,
        }
    }

    /// Display symbol.
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Gt => ">",
            Self::Ge => "≥",
            Self::Lt => "<",
            Self::Le => "≤",
        }
    }
}

/// One side of an interaction or a single-drug rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrugSelector {
    /// Generic keys from the dictionary.
    pub keys: Vec<String>,
    /// Therapeutic classes from the dictionary.
    pub classes: Vec<String>,
}

impl DrugSelector {
    /// Match generic keys only.
    pub fn keys(keys: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|s| (*s).to_string()).collect(),
            classes: Vec::new(),
        }
    }

    /// Match therapeutic classes only.
    pub fn classes(classes: &[&str]) -> Self {
        Self {
            keys: Vec::new(),
            classes: classes.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Match either keys or classes.
    pub fn any(keys: &[&str], classes: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|s| (*s).to_string()).collect(),
            classes: classes.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// True when the order's generic key or class set matches.
    pub fn matches(&self, key: Option<&str>, classes: &[String]) -> bool {
        key.is_some_and(|key| self.keys.iter().any(|candidate| candidate == key))
            || self
                .classes
                .iter()
                .any(|candidate| classes.iter().any(|class| class == candidate))
    }

    /// True when the selector has at least one key or class.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty() && self.classes.is_empty()
    }

    /// Human-readable selector summary for settings.
    pub fn label_th(&self) -> String {
        let mut parts = Vec::new();
        for key in &self.keys {
            parts.push(
                crate::dictionary::all_profiles()
                    .iter()
                    .find(|p| p.key == key)
                    .map(|p| {
                        if p.name_th.is_empty() {
                            p.name_en.to_string()
                        } else {
                            p.name_th.to_string()
                        }
                    })
                    .unwrap_or_else(|| key.clone()),
            );
        }
        for class in &self.classes {
            parts.push(crate::dictionary::class_label(class).to_string());
        }
        if parts.is_empty() {
            "ยา".to_string()
        } else {
            parts.join(" หรือ ")
        }
    }
}

/// Patient-specific condition for a single-drug rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PatientCondition {
    /// Age at least N years.
    AgeAtLeast {
        /// Years.
        years: u8,
    },
    /// Age at most N years.
    AgeAtMost {
        /// Years.
        years: u8,
    },
    /// eGFR below N.
    EgfrBelow {
        /// mL/min/1.73 m².
        value: f64,
    },
    /// Cockcroft-Gault CrCl below N.
    CrclBelow {
        /// mL/min.
        value: f64,
    },
    /// A diagnosis whose ICD-10 code starts with one of the prefixes.
    Diagnosis {
        /// ICD-10 prefixes.
        prefixes: Vec<String>,
        /// Thai condition label.
        label: String,
    },
    /// Pregnant patient.
    Pregnancy,
}

impl PatientCondition {
    /// Thai summary for the rule list.
    pub fn label_th(&self) -> String {
        match self {
            Self::AgeAtLeast { years } => format!("อายุ ≥ {years} ปี"),
            Self::AgeAtMost { years } => format!("อายุ ≤ {years} ปี"),
            Self::EgfrBelow { value } => format!("eGFR < {value:.0}"),
            Self::CrclBelow { value } => format!("CrCl < {value:.0}"),
            Self::Diagnosis { label, .. } => format!("วินิจฉัย: {label}"),
            Self::Pregnancy => "ตั้งครรภ์".to_string(),
        }
    }

    /// Threshold metadata, when the condition has an editable number.
    pub fn threshold_meta(&self) -> Option<ThresholdMeta> {
        match self {
            Self::AgeAtLeast { years } | Self::AgeAtMost { years } => Some(ThresholdMeta {
                label: "เกณฑ์อายุ".into(),
                unit: "ปี".into(),
                default_value: *years as f64,
                min: 0.0,
                max: 120.0,
                step: 1.0,
            }),
            Self::EgfrBelow { value } => Some(ThresholdMeta {
                label: "eGFR ต่ำกว่า".into(),
                unit: "mL/min/1.73m²".into(),
                default_value: *value,
                min: 1.0,
                max: 120.0,
                step: 1.0,
            }),
            Self::CrclBelow { value } => Some(ThresholdMeta {
                label: "CrCl ต่ำกว่า".into(),
                unit: "mL/min".into(),
                default_value: *value,
                min: 1.0,
                max: 150.0,
                step: 1.0,
            }),
            Self::Diagnosis { .. } | Self::Pregnancy => None,
        }
    }

    /// Apply a numeric threshold override.
    pub fn set_threshold(&mut self, value: f64) {
        match self {
            Self::AgeAtLeast { years } | Self::AgeAtMost { years } => {
                *years = value.round().clamp(0.0, 120.0) as u8;
            }
            Self::EgfrBelow { value: threshold } => *threshold = value,
            Self::CrclBelow { value: threshold } => *threshold = value,
            Self::Diagnosis { .. } | Self::Pregnancy => {}
        }
    }
}

/// Editable numeric metadata shown in settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThresholdMeta {
    /// Field label.
    pub label: String,
    /// Unit.
    pub unit: String,
    /// Default value.
    pub default_value: f64,
    /// Minimum allowed.
    pub min: f64,
    /// Maximum allowed.
    pub max: f64,
    /// Input step.
    pub step: f64,
}

/// What makes a rule fire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Trigger {
    /// A pair of distinct orders.
    DrugPair {
        /// First drug.
        a: DrugSelector,
        /// Second drug.
        b: DrugSelector,
    },
    /// A triple of distinct orders.
    DrugTriple {
        /// First drug.
        a: DrugSelector,
        /// Second drug.
        b: DrugSelector,
        /// Third drug.
        c: DrugSelector,
    },
    /// Two or more orders in the same therapeutic class.
    DuplicateClass {
        /// Class key.
        class_key: String,
        /// Thai class label.
        class_label: String,
    },
    /// Two or more orders sharing a class, framed as a burden rather than duplication.
    ClassBurden {
        /// Class key.
        class_key: String,
        /// Thai class label.
        class_label: String,
        /// Minimum number of orders.
        min_count: usize,
    },
    /// One order plus an optional patient condition.
    SingleDrug {
        /// Drug selector.
        drug: DrugSelector,
        /// Patient condition.
        condition: Option<PatientCondition>,
    },
    /// One order plus a lab threshold.
    LabThreshold {
        /// Drug selector.
        drug: DrugSelector,
        /// Lab kind.
        lab: LabKind,
        /// Comparison.
        op: CompareOp,
        /// Threshold value.
        threshold: f64,
    },
    /// One order's estimated mg/day above a maximum.
    MaxDailyDose {
        /// Drug selector.
        drug: DrugSelector,
        /// Maximum mg/day.
        max_mg_per_day: f64,
        /// Only apply at or above this age.
        age_min: Option<u8>,
    },
    /// One order plus an allergy history term and a cross-reactivity class.
    Allergy {
        /// Normalized allergen terms (lowercase).
        allergen_terms: Vec<String>,
        /// Drugs/classes that may cross-react.
        cross: DrugSelector,
    },
}

impl Trigger {
    /// Threshold metadata for the rule settings UI.
    pub fn threshold_meta(&self) -> Option<ThresholdMeta> {
        match self {
            Self::SingleDrug { condition, .. } => {
                condition.as_ref().and_then(|c| c.threshold_meta())
            }
            Self::LabThreshold {
                lab, op, threshold, ..
            } => Some(ThresholdMeta {
                label: format!("{} {} เกณฑ์", lab.label_th(), op.symbol()),
                unit: lab.unit().to_string(),
                default_value: *threshold,
                min: 0.0,
                max: 1000.0,
                step: 0.1,
            }),
            Self::MaxDailyDose { max_mg_per_day, .. } => Some(ThresholdMeta {
                label: "ขนาดสูงสุดต่อวัน".into(),
                unit: "mg/วัน".into(),
                default_value: *max_mg_per_day,
                min: 1.0,
                max: 20000.0,
                step: 1.0,
            }),
            _ => None,
        }
    }

    /// Apply a threshold override to a mutable trigger.
    pub fn set_threshold(&mut self, value: f64) {
        match self {
            Self::SingleDrug {
                condition: Some(condition),
                ..
            } => condition.set_threshold(value),
            Self::SingleDrug {
                condition: None, ..
            } => {}
            Self::LabThreshold { threshold, .. } => *threshold = value,
            Self::MaxDailyDose { max_mg_per_day, .. } => *max_mg_per_day = value,
            _ => {}
        }
    }

    /// Current threshold value, if editable.
    pub fn threshold_value(&self) -> Option<f64> {
        self.threshold_meta().map(|meta| match self {
            Self::SingleDrug { condition, .. } => condition
                .as_ref()
                .and_then(|c| c.threshold_meta())
                .map(|m| m.default_value)
                .unwrap_or(meta.default_value),
            _ => meta.default_value,
        })
    }

    /// Thai summary of the trigger, used in settings.
    pub fn label_th(&self) -> String {
        match self {
            Self::DrugPair { a, b } => format!("{} + {}", a.label_th(), b.label_th()),
            Self::DrugTriple { a, b, c } => {
                format!("{} + {} + {}", a.label_th(), b.label_th(), c.label_th())
            }
            Self::DuplicateClass { class_label, .. } => format!("กลุ่ม {class_label}"),
            Self::ClassBurden { class_label, .. } => format!("กลุ่ม {class_label}"),
            Self::SingleDrug { drug, condition } => match condition {
                Some(condition) => format!("{} เมื่อ{}", drug.label_th(), condition.label_th()),
                None => drug.label_th(),
            },
            Self::LabThreshold {
                drug,
                lab,
                op,
                threshold,
            } => format!(
                "{} เมื่อ {} {} {}",
                drug.label_th(),
                lab.label_th(),
                op.symbol(),
                crate::thai::format_qty(*threshold)
            ),
            Self::MaxDailyDose {
                drug,
                max_mg_per_day,
                ..
            } => format!(
                "{} เกิน {} mg/วัน",
                drug.label_th(),
                crate::thai::format_qty(*max_mg_per_day)
            ),
            Self::Allergy {
                allergen_terms,
                cross,
            } => format!(
                "ประวัติแพ้ {} → {}",
                allergen_terms.join(" / "),
                cross.label_th()
            ),
        }
    }
}

/// One clinical rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    /// Stable id.
    pub id: String,
    /// Category.
    pub category: Category,
    /// Default severity.
    pub severity: Severity,
    /// Short Thai title.
    pub title: String,
    /// What the rule detects.
    pub detail: String,
    /// What to do.
    pub recommendation: String,
    /// Mechanism / evidence.
    pub evidence: String,
    /// Source citation.
    pub source: String,
    /// Trigger.
    pub trigger: Trigger,
    /// Whether the rule is enabled before site overrides.
    pub enabled_by_default: bool,
    /// Whether the UI requires acknowledgement.
    pub requires_ack: bool,
}

impl Rule {
    /// Apply a site override.
    pub fn apply_override(&mut self, override_value: &RuleOverride) {
        self.enabled_by_default = override_value.enabled;
        if let Some(severity) = override_value.severity {
            self.severity = severity;
            self.requires_ack = severity.requires_ack();
        }
        if let Some(threshold) = override_value.threshold {
            self.trigger.set_threshold(threshold);
        }
    }

    /// Threshold metadata for the settings UI.
    pub fn threshold_meta(&self) -> Option<ThresholdMeta> {
        self.trigger.threshold_meta()
    }

    /// Current threshold value for the settings UI.
    pub fn threshold_value(&self) -> Option<f64> {
        self.trigger.threshold_value()
    }
}

/// A site override for one rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RuleOverride {
    /// Enabled state.
    pub enabled: bool,
    /// Severity override.
    pub severity: Option<Severity>,
    /// Numeric threshold override.
    pub threshold: Option<f64>,
}

impl Default for RuleOverride {
    fn default() -> Self {
        Self {
            enabled: true,
            severity: None,
            threshold: None,
        }
    }
}

/// A pharmacist's sign-off for a rule-pack version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulePackSignoff {
    /// Pharmacist name.
    pub pharmacist_name: String,
    /// Professional licence number.
    pub license_no: String,
    /// Sign-off date.
    pub signed_on: NaiveDate,
    /// Rule-pack version signed.
    pub pack_version: String,
    /// Optional note.
    pub note: String,
}

/// Non-secret, site-editable review settings. Stored as plain JSON; never
/// contains patient data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ReviewSettings {
    /// Site label.
    pub site_name: String,
    /// Enabled categories.
    pub enabled_categories: Vec<Category>,
    /// Hide findings below this severity.
    pub min_severity: Severity,
    /// Age at which geriatric rules apply.
    pub elderly_age: u8,
    /// Preferred renal metric.
    pub renal_metric: RenalMetric,
    /// Run pregnancy rules.
    pub enable_pregnancy: bool,
    /// Run pediatric rules.
    pub enable_pediatric: bool,
    /// Show high-alert checklist findings.
    pub show_high_alert: bool,
    /// Finding sort order.
    pub finding_sort: FindingSort,
    /// Per-rule overrides.
    pub overrides: BTreeMap<String, RuleOverride>,
    /// Rule-pack sign-off.
    pub signoff: Option<RulePackSignoff>,
}

fn default_categories() -> Vec<Category> {
    Category::all().to_vec()
}

impl Default for ReviewSettings {
    fn default() -> Self {
        Self {
            site_name: String::new(),
            enabled_categories: default_categories(),
            min_severity: Severity::Info,
            elderly_age: 65,
            renal_metric: RenalMetric::Auto,
            enable_pregnancy: true,
            enable_pediatric: true,
            show_high_alert: true,
            finding_sort: FindingSort::Severity,
            overrides: BTreeMap::new(),
            signoff: None,
        }
    }
}

impl ReviewSettings {
    /// Clamp and de-duplicate values after loading from disk or IPC.
    pub fn normalize(&mut self) {
        self.elderly_age = self.elderly_age.clamp(18, 120);
        if self.enabled_categories.is_empty() {
            self.enabled_categories = default_categories();
        }
        self.enabled_categories.sort_by_key(|c| c.code());
        self.enabled_categories.dedup();
        self.site_name = self.site_name.trim().to_string();
        for override_value in self.overrides.values_mut() {
            if let Some(threshold) = override_value.threshold
                && !threshold.is_finite()
            {
                override_value.threshold = None;
            }
        }
    }

    /// True if a category is enabled.
    pub fn category_enabled(&self, category: Category) -> bool {
        self.enabled_categories.contains(&category)
    }

    /// Effective enabled state for a rule.
    pub fn rule_enabled(&self, rule: &Rule) -> bool {
        if !self.category_enabled(rule.category) {
            return false;
        }
        if rule.category == Category::HighAlert && !self.show_high_alert {
            return false;
        }
        self.overrides
            .get(&rule.id)
            .map(|override_value| override_value.enabled)
            .unwrap_or(rule.enabled_by_default)
    }
}

/// A complete rule pack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulePack {
    /// Stable id.
    pub id: String,
    /// Human name.
    pub name: String,
    /// Version.
    pub version: String,
    /// Effective date.
    pub effective_date: NaiveDate,
    /// Description and disclaimer.
    pub description: String,
    /// Rules.
    pub rules: Vec<Rule>,
}

impl RulePack {
    /// Metadata for the UI, including sign-off state.
    pub fn info(&self, settings: &ReviewSettings) -> RulePackInfo {
        let signed_off = settings
            .signoff
            .as_ref()
            .is_some_and(|signoff| signoff.pack_version == self.version);
        RulePackInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            effective_date: self.effective_date,
            description: self.description.clone(),
            rule_count: self.rules.len(),
            signed_off,
            signoff: settings.signoff.clone(),
        }
    }

    /// Rules with site overrides applied.
    pub fn effective_rules(&self, settings: &ReviewSettings) -> Vec<Rule> {
        self.rules
            .iter()
            .cloned()
            .map(|mut rule| {
                if let Some(override_value) = settings.overrides.get(&rule.id) {
                    rule.apply_override(override_value);
                }
                rule
            })
            .collect()
    }

    /// Rule list for the settings UI.
    pub fn config_views(&self, settings: &ReviewSettings) -> Vec<RuleConfigView> {
        self.effective_rules(settings)
            .into_iter()
            .map(|rule| {
                let enabled = settings.rule_enabled(&rule);
                let default_enabled = rule.enabled_by_default;
                let threshold_meta = rule.threshold_meta();
                let threshold_value = rule.threshold_value();
                let default_severity = self
                    .rules
                    .iter()
                    .find(|original| original.id == rule.id)
                    .map(|original| original.severity)
                    .unwrap_or(rule.severity);
                RuleConfigView {
                    id: rule.id,
                    category: rule.category,
                    title: rule.title,
                    detail: rule.detail,
                    source: rule.source,
                    severity: rule.severity,
                    default_severity,
                    threshold_meta,
                    threshold_value,
                    enabled,
                    default_enabled,
                    requires_ack: rule.requires_ack,
                }
            })
            .collect()
    }
}

/// A rule row returned to the settings UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleConfigView {
    /// Rule id.
    pub id: String,
    /// Category.
    pub category: Category,
    /// Title.
    pub title: String,
    /// Detail.
    pub detail: String,
    /// Source.
    pub source: String,
    /// Effective severity.
    pub severity: Severity,
    /// Pack default severity.
    pub default_severity: Severity,
    /// Editable threshold metadata.
    pub threshold_meta: Option<ThresholdMeta>,
    /// Current threshold value.
    pub threshold_value: Option<f64>,
    /// Whether the rule is enabled after category checks.
    pub enabled: bool,
    /// Whether the rule is enabled before the category check.
    pub default_enabled: bool,
    /// Whether acknowledgement is required.
    pub requires_ack: bool,
}

// ---------------------------------------------------------------------------
// Default pack
// ---------------------------------------------------------------------------

/// The bundled starter pack. Every rule must be reviewed by the site
/// pharmacist before clinical use.
pub fn default_rule_pack() -> RulePack {
    RulePack {
        id: "drug-ya-starter".into(),
        name: "Drug-Ya Starter Review Rules".into(),
        version: "2026.11.0".into(),
        effective_date: NaiveDate::from_ymd_opt(2026, 11, 4).expect("valid date"),
        description: "ชุดกฎเริ่มต้นสำหรับทบทวนคำสั่งยาวันนี้ จัดทำเพื่อให้เภสัชกรปรับและรับรองก่อนใช้งานจริง ไม่ใช่ฐานข้อมูลอันตรกิริยาทั้งหมด และไม่แทนการตัดสินใจทางคลินิก".into(),
        rules: starter_rules(),
    }
}

#[allow(clippy::vec_init_then_push)]
fn starter_rules() -> Vec<Rule> {
    let mut rules = Vec::new();

    // ---- Allergy cross-reactivity -----------------------------------------
    rules.push(rule(
        "allergy.penicillin.cephalosporin",
        Category::Allergy,
        Severity::Moderate,
        "ประวัติแพ้ penicillin กับยาผู้ป่วยกลุ่ม cephalosporin",
        "ผู้ป่วยมีประวัติแพ้ยากลุ่ม penicillin และมีคำสั่งยากลุ่ม cephalosporin ซึ่งอาจมี cross-reactivity ได้บ้าง แม้ความเสี่ยงจะต่ำในคนที่แพ้แบบไม่รุนแรง",
        "ทบทวนลักษณะการแพ้เดิมและความรุนแรง หากเคยมี anaphylaxis/angioedema/SJS/TEN ให้หลีกเลี่ยงหรือปรึกษาแพทย์ก่อนจ่าย",
        "Cross-reactivity ระหว่าง penicillin กับ cephalosporin พบได้น้อย แต่สูงขึ้นใน cephalosporin รุ่นแรกและในผู้ที่แพ้แบบรุนแรง",
        "Drug allergy label; AAO/AAAAI guidance; starter rule",
        Trigger::Allergy {
            allergen_terms: vec!["penicillin".into(), "penicillins".into(), "เพนิซิลลิน".into()],
            cross: DrugSelector::classes(&["cephalosporin"]),
        },
    ));
    rules.push(rule(
        "allergy.sulfonamide",
        Category::Allergy,
        Severity::High,
        "ประวัติแพ้ sulfonamide กับยาที่อาจ cross-react",
        "ผู้ป่วยมีประวัติแพ้ sulfonamide และมีคำสั่งยาที่อยู่ในกลุ่ม sulfonamide",
        "ยืนยันตัวยาที่แพ้และอาการเดิม หลีกเลี่ยง sulfonamide ที่ไม่จำเป็น หรือปรึกษาแพทย์เพื่อเลือกทางเลือก",
        "Sulfonamide antibiotic allergy is a class label; not all sulfonamide-containing drugs cross-react.",
        "Drug allergy label; starter rule",
        Trigger::Allergy {
            allergen_terms: vec!["sulfonamide".into(), "sulfa".into(), "ซัลฟา".into()],
            cross: DrugSelector::classes(&["sulfonamide"]),
        },
    ));
    rules.push(rule(
        "allergy.nsaid",
        Category::Allergy,
        Severity::High,
        "ประวัติแพ้ NSAIDs กับยาที่อาจ cross-react",
        "ผู้ป่วยมีประวัติแพ้ยากลุ่ม NSAIDs และมีคำสั่งยา NSAID/aspirin",
        "หลีกเลี่ยง NSAID ที่ไม่จำเป็น หากต้องใช้ให้เลือกตามลักษณะการแพ้เดิมและปรึกษาแพทย์/ผู้เชี่ยวชาญ",
        "NSAID hypersensitivity can be class-wide, especially in aspirin-exacerbated respiratory disease.",
        "Drug allergy label; starter rule",
        Trigger::Allergy {
            allergen_terms: vec!["nsaid".into(), "nsaids".into(), "ยากลุ่ม nsaid".into()],
            cross: DrugSelector::classes(&["nsaid"]),
        },
    ));
    rules.push(rule(
        "allergy.cephalosporin",
        Category::Allergy,
        Severity::High,
        "ประวัติแพ้ cephalosporin กับยาชนิดเดียวกัน",
        "ผู้ป่วยมีประวัติแพ้ยากลุ่ม cephalosporin และมีคำสั่งยากลุ่มเดียวกัน",
        "ยืนยันการแพ้และเลือกยาทางเลือกที่ไม่ใช่กลุ่มนี้",
        "Direct class allergy.",
        "Drug allergy label; starter rule",
        Trigger::Allergy {
            allergen_terms: vec!["cephalosporin".into(), "เซฟา".into()],
            cross: DrugSelector::classes(&["cephalosporin"]),
        },
    ));
    rules.push(rule(
        "allergy.macrolide",
        Category::Allergy,
        Severity::Moderate,
        "ประวัติแพ้ macrolide กับยาที่อาจ cross-react",
        "ผู้ป่วยมีประวัติแพ้ยากลุ่ม macrolide และมีคำสั่งยาในกลุ่มเดียวกัน",
        "ยืนยันอาการเดิมและพิจารณายากลุ่มอื่น",
        "Class allergy; cross-reactivity within macrolides is possible.",
        "Drug allergy label; starter rule",
        Trigger::Allergy {
            allergen_terms: vec!["macrolide".into()],
            cross: DrugSelector::classes(&["macrolide"]),
        },
    ));
    rules.push(rule(
        "allergy.fluoroquinolone",
        Category::Allergy,
        Severity::Moderate,
        "ประวัติแพ้ fluoroquinolone กับยาที่อาจ cross-react",
        "ผู้ป่วยมีประวัติแพ้ยากลุ่ม fluoroquinolone และมีคำสั่งยาในกลุ่มเดียวกัน",
        "ยืนยันอาการเดิมและพิจารณายากลุ่มอื่น",
        "Class allergy.",
        "Drug allergy label; starter rule",
        Trigger::Allergy {
            allergen_terms: vec!["fluoroquinolone".into()],
            cross: DrugSelector::classes(&["fluoroquinolone"]),
        },
    ));
    rules.push(rule(
        "allergy.opioid",
        Category::Allergy,
        Severity::High,
        "ประวัติแพ้ opioid กับยาที่อาจ cross-react",
        "ผู้ป่วยมีประวัติแพ้ยากลุ่ม opioid และมีคำสั่งยา opioid",
        "แยกอาการแพ้จริงออกจากผลข้างเคียง (คลื่นไส้/คัน) และเลือกทางเลือกที่เหมาะสม",
        "Opioid allergy labels are often intolerance; true allergy requires review.",
        "Drug allergy label; starter rule",
        Trigger::Allergy {
            allergen_terms: vec!["opioid".into(), "โอปิออยด์".into()],
            cross: DrugSelector::classes(&["opioid"]),
        },
    ));
    rules.push(rule(
        "allergy.tetracycline",
        Category::Allergy,
        Severity::Moderate,
        "ประวัติแพ้ tetracycline กับยาที่อาจ cross-react",
        "ผู้ป่วยมีประวัติแพ้ยากลุ่ม tetracycline และมีคำสั่งยาในกลุ่มเดียวกัน",
        "ยืนยันอาการเดิมและพิจารณายากลุ่มอื่น",
        "Class allergy.",
        "Drug allergy label; starter rule",
        Trigger::Allergy {
            allergen_terms: vec!["tetracycline".into()],
            cross: DrugSelector::classes(&["tetracycline"]),
        },
    ));

    // ---- Drug-drug interactions -------------------------------------------
    ddi(
        &mut rules,
        "ddi.warfarin.nsaid",
        Severity::High,
        "warfarin + NSAID",
        "เพิ่มความเสี่ยงเลือดออกในทางเดินอาหารและที่อื่น",
        "หลีกเลี่ยง NSAID หากทำได้ หรือใช้ระยะสั้นที่สุดพร้อมมาตรการป้องกันกระเพาะและติดตามอาการเลือดออก",
        "NSAIDs ยับยั้งเกล็ดเลือดและระคายกระเพาะ ร่วมกับ warfarin ที่เพิ่ม INR",
        "Warfarin label; ACC/AHA; starter rule",
        DrugSelector::keys(&["warfarin"]),
        DrugSelector::classes(&["nsaid"]),
    );
    ddi(
        &mut rules,
        "ddi.warfarin.antiplatelet",
        Severity::High,
        "warfarin + ยาต้านเกล็ดเลือด",
        "เพิ่มความเสี่ยงเลือดออก โดยเฉพาะเมื่อใช้ร่วมกับ aspirin หรือ clopidogrel",
        "ยืนยันข้อบ่งใช้ร่วมและระยะเวลาที่จำเป็น ติดตามอาการเลือดออกและ INR ตามแผน",
        "ผลเสริมกันของ anticoagulant และ antiplatelet",
        "Warfarin label; starter rule",
        DrugSelector::keys(&["warfarin"]),
        DrugSelector::classes(&["antiplatelet"]),
    );
    ddi(
        &mut rules,
        "ddi.warfarin.amiodarone",
        Severity::High,
        "warfarin + amiodarone",
        "amiodarone เพิ่มระดับ warfarin และ INR อาจสูงขึ้นช้าๆ ภายในหลายวัน",
        "ติดตาม INR อย่างใกล้ชิดหลังเริ่ม/หยุด amiodarone และปรับขนาด warfarin ตามผล",
        "CYP2C9 และ CYP3A4 inhibition ลด metabolism ของ warfarin",
        "Warfarin/amiodarone labels; starter rule",
        DrugSelector::keys(&["warfarin"]),
        DrugSelector::keys(&["amiodarone"]),
    );
    ddi(
        &mut rules,
        "ddi.warfarin.fluconazole",
        Severity::High,
        "warfarin + fluconazole",
        "fluconazole เพิ่มระดับ warfarin และความเสี่ยงเลือดออก",
        "ติดตาม INR และพิจารณาลดขนาด warfarin ระหว่างใช้ fluconazole",
        "CYP2C9 inhibition",
        "Warfarin/fluconazole labels; starter rule",
        DrugSelector::keys(&["warfarin"]),
        DrugSelector::keys(&["fluconazole"]),
    );
    ddi(
        &mut rules,
        "ddi.warfarin.metronidazole",
        Severity::High,
        "warfarin + metronidazole",
        "metronidazole เพิ่มระดับ warfarin และ INR",
        "ติดตาม INR และปรับขนาด warfarin ตามผล",
        "CYP2C9 inhibition",
        "Warfarin/metronidazole labels; starter rule",
        DrugSelector::keys(&["warfarin"]),
        DrugSelector::keys(&["metronidazole"]),
    );
    ddi(
        &mut rules,
        "ddi.warfarin.cotrimoxazole",
        Severity::High,
        "warfarin + co-trimoxazole",
        "เพิ่มระดับ warfarin และความเสี่ยงเลือดออก",
        "ติดตาม INR อย่างใกล้ชิด พิจารณายาปฏิชีวนะทางเลือก",
        "CYP2C9 inhibition และการลด vitamin K จากเชื้อในลำไส้",
        "Warfarin/TMP-SMX labels; starter rule",
        DrugSelector::keys(&["warfarin"]),
        DrugSelector::keys(&["co_trimoxazole"]),
    );
    ddi(
        &mut rules,
        "ddi.warfarin.macrolide",
        Severity::High,
        "warfarin + clarithromycin/erythromycin",
        "macrolide บางชนิดเพิ่มระดับ warfarin และ INR",
        "ติดตาม INR และพิจารณา azithromycin หรือยาปฏิชีวนะทางเลือกตามความเหมาะสม",
        "CYP3A4 inhibition",
        "Warfarin/macrolide labels; starter rule",
        DrugSelector::keys(&["warfarin"]),
        DrugSelector::keys(&["clarithromycin", "erythromycin"]),
    );
    ddi(
        &mut rules,
        "ddi.warfarin.rifampicin",
        Severity::High,
        "warfarin + rifampicin",
        "rifampicin กระตุ้น metabolism ของ warfarin ทำให้ INR และประสิทธิภาพลดลง",
        "ติดตาม INR หลังเริ่ม/หยุด rifampicin และปรับขนาด warfarin ตามผล",
        "CYP3A4/CYP2C9 induction",
        "Warfarin/rifampicin labels; starter rule",
        DrugSelector::keys(&["warfarin"]),
        DrugSelector::keys(&["rifampicin", "rifabutin"]),
    );
    ddi(
        &mut rules,
        "ddi.warfarin.antiepileptic",
        Severity::High,
        "warfarin + carbamazepine/phenytoin/phenobarbital",
        "ยาต้านชักบางชนิดเปลี่ยนระดับ warfarin ได้ทั้งเพิ่มและลด",
        "ติดตาม INR อย่างใกล้ชิดเมื่อเริ่ม ปรับ หรือหยุดยา",
        "CYP induction/inhibition ที่ซับซ้อน",
        "Warfarin label; starter rule",
        DrugSelector::keys(&["warfarin"]),
        DrugSelector::keys(&["carbamazepine", "phenytoin", "phenobarbital"]),
    );

    ddi(
        &mut rules,
        "ddi.doac.strong_inhibitor",
        Severity::High,
        "DOAC + strong CYP3A4/P-gp inhibitor",
        "อาจเพิ่มระดับ DOAC และความเสี่ยงเลือดออก",
        "หลีกเลี่ยงหรือปรับขนาดตาม label ของ DOAC แต่ละชนิด และติดตามอาการเลือดออก",
        "CYP3A4/P-gp inhibition เพิ่ม exposure ของ DOAC",
        "DOAC labels; starter rule",
        DrugSelector::classes(&["doac"]),
        DrugSelector::any(&["clarithromycin"], &["azole"]),
    );
    ddi(
        &mut rules,
        "ddi.doac.rifampicin",
        Severity::High,
        "DOAC + rifampicin",
        "rifampicin ลดระดับ DOAC และอาจลดประสิทธิภาพป้องกันลิ่มเลือด",
        "หลีกเลี่ยงการใช้ร่วม หรือเลือก anticoagulant ที่เหมาะสมหลังปรึกษาแพทย์",
        "CYP3A4/P-gp induction",
        "DOAC/rifampicin labels; starter rule",
        DrugSelector::classes(&["doac"]),
        DrugSelector::keys(&["rifampicin", "rifabutin"]),
    );
    ddi(
        &mut rules,
        "ddi.doac.antiepileptic",
        Severity::High,
        "DOAC + carbamazepine/phenytoin",
        "ยาต้านชักที่กระตุ้น CYP3A4/P-gp อาจลดระดับ DOAC",
        "หลีกเลี่ยงหรือเลือก anticoagulant ทางเลือกหลังประเมินความเสี่ยง",
        "CYP3A4/P-gp induction",
        "DOAC labels; starter rule",
        DrugSelector::classes(&["doac"]),
        DrugSelector::keys(&["carbamazepine", "phenytoin", "phenobarbital"]),
    );
    ddi(
        &mut rules,
        "ddi.doac.nsaid",
        Severity::High,
        "DOAC + NSAID",
        "เพิ่มความเสี่ยงเลือดออกในทางเดินอาหาร",
        "หลีกเลี่ยง NSAID หากทำได้ หรือใช้ระยะสั้นที่สุดพร้อมป้องกันกระเพาะ",
        "ผลเสริมกันต่อการแข็งตัวของเลือดและเยื่อบุกระเพาะ",
        "DOAC labels; starter rule",
        DrugSelector::classes(&["doac"]),
        DrugSelector::classes(&["nsaid"]),
    );
    ddi(
        &mut rules,
        "ddi.antiplatelet.nsaid",
        Severity::High,
        "ยาต้านเกล็ดเลือด + NSAID",
        "เพิ่มความเสี่ยงเลือดออกในทางเดินอาหาร",
        "หลีกเลี่ยง NSAID หรือใช้ระยะสั้นที่สุดพร้อมมาตรการป้องกัน",
        "Antiplatelet effect + mucosal injury",
        "Antiplatelet/NSAID labels; starter rule",
        DrugSelector::classes(&["antiplatelet"]),
        DrugSelector::classes(&["nsaid"]),
    );
    ddi(
        &mut rules,
        "ddi.aspirin.other_nsaid",
        Severity::High,
        "aspirin + NSAID อื่น",
        "เพิ่มความเสี่ยงเลือดออก และ ibuprofen อาจรบกวนฤทธิ์ต้านเกล็ดเลือดของ aspirin ขนาดต่ำ",
        "แยกเวลาหรือหลีกเลี่ยง NSAID ตามข้อบ่งใช้ และประเมินความเสี่ยงเลือดออก",
        "Competitive COX-1 binding และผลต่อเยื่อบุกระเพาะ",
        "FDA aspirin/NSAID interaction; starter rule",
        DrugSelector::keys(&["aspirin"]),
        DrugSelector::classes(&["nsaid_non_aspirin"]),
    );

    ddi(
        &mut rules,
        "ddi.ras.ksparing",
        Severity::High,
        "ACEI/ARB + potassium-sparing diuretic",
        "เพิ่มความเสี่ยง hyperkalemia",
        "ตรวจ potassium และ creatinine ก่อนเริ่มและหลังปรับขนาด เฝ้าระวังอาการอ่อนแรง/หัวใจเต้นผิดจังหวะ",
        "ผลเสริมกันในการเก็บโพแทสเซียม",
        "ACEI/ARB and spironolactone labels; starter rule",
        DrugSelector::classes(&["acei", "arb"]),
        DrugSelector::classes(&["ksparing_diuretic"]),
    );
    ddi(
        &mut rules,
        "ddi.ras.potassium",
        Severity::High,
        "ACEI/ARB + potassium supplement",
        "เพิ่มความเสี่ยง hyperkalemia",
        "ตรวจ potassium และพิจารณาหยุดหรือลด potassium supplement ที่ไม่จำเป็น",
        "ผลเสริมกันในการเพิ่มโพแทสเซียม",
        "ACEI/ARB labels; starter rule",
        DrugSelector::classes(&["acei", "arb"]),
        DrugSelector::keys(&["potassium_chloride"]),
    );
    ddi(
        &mut rules,
        "ddi.ras.nsaid",
        Severity::Moderate,
        "ACEI/ARB + NSAID",
        "NSAID อาจลดผลของ ACEI/ARB และเพิ่มความเสี่ยงไตเสื่อม โดยเฉพาะเมื่อขาดน้ำ",
        "ใช้ NSAID ระยะสั้นที่สุดและติดตาม creatinine/potassium ถ้าจำเป็น",
        "Prostaglandin inhibition ลด renal perfusion",
        "ACEI/ARB and NSAID labels; starter rule",
        DrugSelector::classes(&["acei", "arb"]),
        DrugSelector::classes(&["nsaid"]),
    );
    rules.push(rule(
        "ddi.triple_whammy",
        Category::Interaction,
        Severity::High,
        "ACEI/ARB + NSAID + diuretic (triple whammy)",
        "การใช้ยาสามกลุ่มร่วมกันเพิ่มความเสี่ยง acute kidney injury",
        "หลีกเลี่ยง NSAID หากทำได้ ตรวจ creatinine และติดตามปัสสาวะ/ hydration",
        "NSAID ลด renal blood flow ขณะที่ ACEI/ARB และ diuretic ลด effective volume",
        "KDIGO/tertiary references; starter rule",
        Trigger::DrugTriple {
            a: DrugSelector::classes(&["acei", "arb"]),
            b: DrugSelector::classes(&["nsaid"]),
            c: DrugSelector::classes(&["loop_diuretic", "thiazide"]),
        },
    ));
    ddi(
        &mut rules,
        "ddi.ksparing.potassium",
        Severity::High,
        "spironolactone/eplerenone + potassium supplement",
        "เพิ่มความเสี่ยง hyperkalemia รุนแรง",
        "หลีกเลี่ยงการเสริม potassium โดยไม่จำเป็น และตรวจ potassium ตามแผน",
        "ผลเสริมกันในการเก็บโพแทสเซียม",
        "Spironolactone label; starter rule",
        DrugSelector::classes(&["ksparing_diuretic"]),
        DrugSelector::keys(&["potassium_chloride"]),
    );

    ddi(
        &mut rules,
        "ddi.digoxin.amiodarone",
        Severity::High,
        "digoxin + amiodarone",
        "amiodarone เพิ่มระดับ digoxin และความเสี่ยงพิษจาก digoxin",
        "พิจารณาลดขนาด digoxin ลงประมาณครึ่งหนึ่งเมื่อเริ่ม amiodarone และติดตามระดับ/อาการ",
        "P-gp inhibition ลดการกำจัด digoxin",
        "Digoxin/amiodarone labels; starter rule",
        DrugSelector::keys(&["digoxin"]),
        DrugSelector::keys(&["amiodarone"]),
    );
    ddi(
        &mut rules,
        "ddi.digoxin.verapamil",
        Severity::High,
        "digoxin + verapamil",
        "verapamil เพิ่มระดับ digoxin และอาจ减慢 AV conduction",
        "ติดตามระดับ digoxin และชีพจร/สัญญาณ ECG พิจารณาลดขนาด",
        "P-gp inhibition + AV node effect",
        "Digoxin/verapamil labels; starter rule",
        DrugSelector::keys(&["digoxin"]),
        DrugSelector::keys(&["verapamil"]),
    );
    ddi(
        &mut rules,
        "ddi.digoxin.clarithromycin",
        Severity::High,
        "digoxin + clarithromycin",
        "clarithromycin เพิ่มระดับ digoxin",
        "ติดตามระดับ digoxin และอาการพิษ",
        "P-gp inhibition",
        "Digoxin/clarithromycin labels; starter rule",
        DrugSelector::keys(&["digoxin"]),
        DrugSelector::keys(&["clarithromycin"]),
    );
    ddi(
        &mut rules,
        "ddi.digoxin.diuretic",
        Severity::Moderate,
        "digoxin + loop/thiazide diuretic",
        "ภาวะโพแทสเซียมต่ำจากยาขับปัสสาวะเพิ่มความเสี่ยงพิษจาก digoxin",
        "ติดตาม potassium และ magnesium โดยเฉพาะเมื่อเริ่มหรือเพิ่มขนาด diuretic",
        "Hypokalemia เพิ่ม sensitivity ต่อ digoxin",
        "Digoxin label; starter rule",
        DrugSelector::keys(&["digoxin"]),
        DrugSelector::classes(&["loop_diuretic", "thiazide"]),
    );

    ddi(
        &mut rules,
        "ddi.methotrexate.nsaid",
        Severity::High,
        "methotrexate + NSAID",
        "NSAID ลดการกำจัด methotrexate และเพิ่มพิษต่อไขกระดูก/ไต",
        "หลีกเลี่ยง NSAID หากทำได้ และติดตาม CBC, creatinine, อาการ mucositis",
        "Renal clearance ลดลงและ protein binding displacement",
        "Methotrexate label; starter rule",
        DrugSelector::keys(&["methotrexate"]),
        DrugSelector::classes(&["nsaid"]),
    );
    ddi(
        &mut rules,
        "ddi.methotrexate.cotrimoxazole",
        Severity::Critical,
        "methotrexate + co-trimoxazole",
        "เพิ่มความเสี่ยง bone marrow suppression รุนแรง",
        "หลีกเลี่ยงการใช้ร่วม หรือปรึกษาแพทย์เพื่อยาปฏิชีวนะทางเลือก",
        "Trimethoprim เพิ่ม antifolate effect",
        "Methotrexate/TMP-SMX labels; starter rule",
        DrugSelector::keys(&["methotrexate"]),
        DrugSelector::keys(&["co_trimoxazole"]),
    );
    ddi(
        &mut rules,
        "ddi.lithium.nsaid",
        Severity::High,
        "lithium + NSAID",
        "NSAID ลดการกำจัด lithium และเพิ่มระดับ lithium",
        "หลีกเลี่ยง NSAID หรือติดตามระดับ lithium อย่างใกล้ชิด",
        "Reduced renal prostaglandin synthesis",
        "Lithium/NSAID labels; starter rule",
        DrugSelector::keys(&["lithium"]),
        DrugSelector::classes(&["nsaid"]),
    );
    ddi(
        &mut rules,
        "ddi.lithium.ras",
        Severity::High,
        "lithium + ACEI/ARB",
        "เพิ่มระดับ lithium และความเสี่ยงพิษ",
        "ติดตามระดับ lithium และอาการสั่น/สับสน/เดินเซ",
        "Reduced renal lithium clearance",
        "Lithium/ACEI labels; starter rule",
        DrugSelector::keys(&["lithium"]),
        DrugSelector::classes(&["acei", "arb"]),
    );
    ddi(
        &mut rules,
        "ddi.lithium.thiazide",
        Severity::High,
        "lithium + thiazide",
        "thiazide เพิ่มระดับ lithium อย่างชัดเจน",
        "หลีกเลี่ยงหรือลดขนาด lithium และติดตามระดับยา",
        "Increased proximal lithium reabsorption",
        "Lithium/thiazide labels; starter rule",
        DrugSelector::keys(&["lithium"]),
        DrugSelector::classes(&["thiazide"]),
    );

    ddi(
        &mut rules,
        "ddi.ssri.tramadol",
        Severity::High,
        "SSRI + tramadol",
        "เพิ่มความเสี่ยง serotonin syndrome และ seizure",
        "พิจารณายาแก้ปวดทางเลือก และเฝ้าระวังอาการกระตุก/เหงื่อออก/ตัวสั่น/สับสน",
        "Serotonergic effect + seizure threshold lowering",
        "SSRI/tramadol labels; starter rule",
        DrugSelector::classes(&["ssri"]),
        DrugSelector::keys(&["tramadol"]),
    );
    ddi(
        &mut rules,
        "ddi.ssri.maoi",
        Severity::Critical,
        "SSRI + MAOI",
        "เสี่ยง serotonin syndrome รุนแรง",
        "ห้ามใช้ร่วมกัน และเว้นช่วงตามครึ่งชีวิตของยาแต่ละชนิด",
        "Serotonergic overactivity",
        "SSRI/MAOI labels; starter rule",
        DrugSelector::classes(&["ssri"]),
        DrugSelector::classes(&["maoi"]),
    );
    ddi(
        &mut rules,
        "ddi.ssri.nsaid",
        Severity::Moderate,
        "SSRI + NSAID",
        "เพิ่มความเสี่ยงเลือดออกในทางเดินอาหาร",
        "ใช้ NSAID ระยะสั้นที่สุด พิจารณา gastroprotection และติดตามอาการ",
        "Serotonin reuptake inhibition ลด platelet serotonin",
        "SSRI/NSAID labels; starter rule",
        DrugSelector::classes(&["ssri"]),
        DrugSelector::classes(&["nsaid"]),
    );
    ddi(
        &mut rules,
        "ddi.opioid.benzodiazepine",
        Severity::Critical,
        "opioid + benzodiazepine",
        "เพิ่มความเสี่ยง respiratory depression และการเสียชีวิต",
        "หลีกเลี่ยงการใช้ร่วมหากทำได้ หรือใช้ขนาดต่ำสุดและติดตามการหายใจอย่างใกล้ชิด",
        "CNS/respiratory depression แบบเสริมกัน",
        "FDA boxed warning; starter rule",
        DrugSelector::classes(&["opioid"]),
        DrugSelector::classes(&["benzodiazepine"]),
    );
    ddi(
        &mut rules,
        "ddi.opioid.gabapentinoid",
        Severity::High,
        "opioid + gabapentinoid",
        "เพิ่มความเสี่ยง sedation และ respiratory depression",
        "ใช้ขนาดต่ำสุด เริ่มช้าๆ และติดตามอาการง่วงซึม/หายใจช้า",
        "CNS depression แบบเสริมกัน",
        "FDA warning; starter rule",
        DrugSelector::classes(&["opioid"]),
        DrugSelector::classes(&["gabapentinoid"]),
    );
    ddi(
        &mut rules,
        "ddi.clopidogrel.omeprazole",
        Severity::Moderate,
        "clopidogrel + omeprazole/esomeprazole",
        "ลดการเปลี่ยน clopidogrel เป็น active metabolite และอาจลดประสิทธิภาพ",
        "พิจารณา pantoprazole หรือยาลดกรดกลุ่มอื่นที่ไม่มีผลต่อ CYP2C19",
        "CYP2C19 inhibition",
        "Clopidogrel label; starter rule",
        DrugSelector::keys(&["clopidogrel"]),
        DrugSelector::keys(&["omeprazole", "esomeprazole"]),
    );
    ddi(
        &mut rules,
        "ddi.clopidogrel.strong_inhibitor",
        Severity::Moderate,
        "clopidogrel + strong CYP2C19 inhibitor",
        "อาจลดการเกิด active metabolite ของ clopidogrel",
        "หลีกเลี่ยงหากทำได้ หรือติดตามผลลัพธ์ทางคลินิก",
        "CYP2C19 inhibition",
        "Clopidogrel label; starter rule",
        DrugSelector::keys(&["clopidogrel"]),
        DrugSelector::keys(&["fluvoxamine", "fluconazole"]),
    );
    ddi(
        &mut rules,
        "ddi.statin.strong_inhibitor",
        Severity::High,
        "simvastatin/atorvastatin + strong CYP3A4 inhibitor",
        "เพิ่มระดับ statin และความเสี่ยง myopathy/rhabdomyolysis",
        "หลีกเลี่ยง simvastatin กับยาดังกล่าว หรือเปลี่ยนเป็น statin ที่ไม่ผ่าน CYP3A4 และติดตามอาการปวดกล้ามเนื้อ",
        "CYP3A4 inhibition",
        "Statin labels; starter rule",
        DrugSelector::keys(&["simvastatin", "atorvastatin"]),
        DrugSelector::keys(&[
            "clarithromycin",
            "erythromycin",
            "ketoconazole",
            "itraconazole",
        ]),
    );
    ddi(
        &mut rules,
        "ddi.simvastatin.amiodarone",
        Severity::Moderate,
        "simvastatin + amiodarone",
        "เพิ่มความเสี่ยง myopathy; ขนาด simvastatin ไม่ควรเกิน 20 mg/วันเมื่อใช้ร่วม",
        "จำกัดขนาด simvastatin หรือเปลี่ยน statin",
        "CYP3A4 inhibition",
        "Simvastatin label; starter rule",
        DrugSelector::keys(&["simvastatin"]),
        DrugSelector::keys(&["amiodarone"]),
    );
    ddi(
        &mut rules,
        "ddi.colchicine.strong_inhibitor",
        Severity::Critical,
        "colchicine + clarithromycin/azole",
        "เพิ่มระดับ colchicine อย่างมากและเสี่ยง toxicity รุนแรง",
        "หลีกเลี่ยงหรือลดขนาดอย่างมากตาม label และติดตามอาการกล้ามเนื้อ/เลือด/ทางเดินอาหาร",
        "CYP3A4/P-gp inhibition",
        "Colchicine label; starter rule",
        DrugSelector::keys(&["colchicine"]),
        DrugSelector::any(&["clarithromycin"], &["azole"]),
    );
    ddi(
        &mut rules,
        "ddi.sulfonylurea.insulin",
        Severity::High,
        "sulfonylurea + insulin",
        "เพิ่มความเสี่ยง hypoglycemia",
        "ติดตามน้ำตาลเลือดอย่างใกล้ชิด พิจารณาลดขนาดยาและให้คำแนะนำอาการ hypoglycemia",
        "ผลลดน้ำตาลเสริมกัน",
        "Diabetes guideline; starter rule",
        DrugSelector::classes(&["sulfonylurea"]),
        DrugSelector::classes(&["insulin"]),
    );
    ddi(
        &mut rules,
        "ddi.betablocker.verapamil",
        Severity::High,
        "beta-blocker + verapamil/diltiazem",
        "เพิ่มความเสี่ยง bradycardia, AV block และ hypotension",
        "ติดตามชีพจร/ความดันและ ECG พิจารณาลดขนาดหรือเลี่ยงการใช้ร่วม",
        "ผลเสริมกันต่อ conduction และ contractility",
        "Beta-blocker/CCB labels; starter rule",
        DrugSelector::classes(&["beta_blocker"]),
        DrugSelector::keys(&["verapamil", "diltiazem"]),
    );
    ddi(
        &mut rules,
        "ddi.betablocker.antidiabetic",
        Severity::Moderate,
        "beta-blocker + insulin/sulfonylurea",
        "beta-blocker อาจกลบอาการเตือน hypoglycemia (ใจสั่น/ tremulous)",
        "ให้คำแนะนำการตรวจน้ำตาลและติดตาม hypoglycemia",
        "Masking of adrenergic warning symptoms",
        "Beta-blocker label; starter rule",
        DrugSelector::classes(&["beta_blocker"]),
        DrugSelector::classes(&["insulin", "sulfonylurea"]),
    );
    ddi(
        &mut rules,
        "ddi.nsaid.corticosteroid",
        Severity::High,
        "NSAID + corticosteroid",
        "เพิ่มความเสี่ยงแผล/เลือดออกในทางเดินอาหาร",
        "หลีกเลี่ยง NSAID หากทำได้ หรือใช้ระยะสั้นพร้อม gastroprotection",
        "Mucosal injury แบบเสริมกัน",
        "NSAID/corticosteroid labels; starter rule",
        DrugSelector::classes(&["nsaid"]),
        DrugSelector::classes(&["corticosteroid"]),
    );
    ddi(
        &mut rules,
        "ddi.nsaid.diuretic",
        Severity::Moderate,
        "NSAID + loop/thiazide diuretic",
        "NSAID อาจลดผลขับปัสสาวะและเพิ่มความเสี่ยงไตเสื่อม",
        "ติดตามความดัน น้ำหนัก และ creatinine ระหว่างใช้ร่วม",
        "Reduced renal prostaglandin synthesis",
        "NSAID/diuretic labels; starter rule",
        DrugSelector::classes(&["nsaid"]),
        DrugSelector::classes(&["loop_diuretic", "thiazide"]),
    );
    ddi(
        &mut rules,
        "ddi.theophylline.ciprofloxacin",
        Severity::High,
        "theophylline + ciprofloxacin",
        "ciprofloxacin เพิ่มระดับ theophylline และความเสี่ยง toxicity",
        "ติดตามระดับ theophylline/อาการใจสั่น คลื่นไส้ ชัก และพิจารณาลดขนาด",
        "CYP1A2 inhibition",
        "Theophylline/ciprofloxacin labels; starter rule",
        DrugSelector::keys(&["theophylline", "aminophylline"]),
        DrugSelector::keys(&["ciprofloxacin"]),
    );
    ddi(
        &mut rules,
        "ddi.theophylline.macrolide",
        Severity::High,
        "theophylline + clarithromycin/erythromycin",
        "macrolide เพิ่มระดับ theophylline",
        "ติดตามระดับยา/อาการ toxicity และพิจารณาลดขนาด",
        "CYP3A4/CYP1A2 inhibition",
        "Theophylline/macrolide labels; starter rule",
        DrugSelector::keys(&["theophylline", "aminophylline"]),
        DrugSelector::keys(&["clarithromycin", "erythromycin"]),
    );
    ddi(
        &mut rules,
        "ddi.carbamazepine.macrolide",
        Severity::High,
        "carbamazepine + clarithromycin/erythromycin",
        "เพิ่มระดับ carbamazepine และความเสี่ยง ataxia/ sedation/ ชัก",
        "หลีกเลี่ยงหรือติดตามระดับ carbamazepine และอาการทางระบบประสาท",
        "CYP3A4 inhibition",
        "Carbamazepine/macrolide labels; starter rule",
        DrugSelector::keys(&["carbamazepine"]),
        DrugSelector::keys(&["clarithromycin", "erythromycin"]),
    );
    ddi(
        &mut rules,
        "ddi.phenytoin.fluconazole",
        Severity::High,
        "phenytoin + fluconazole",
        "fluconazole เพิ่มระดับ phenytoin และความเสี่ยง toxicity",
        "ติดตามระดับ phenytoin และอาการเดินเซ/ตาเห็นภาพซ้อน",
        "CYP2C9 inhibition",
        "Phenytoin/fluconazole labels; starter rule",
        DrugSelector::keys(&["phenytoin"]),
        DrugSelector::keys(&["fluconazole"]),
    );
    ddi(
        &mut rules,
        "ddi.valproate.lamotrigine",
        Severity::High,
        "valproate + lamotrigine",
        "valproate เพิ่มระดับ lamotrigine และความเสี่ยงผื่นรุนแรง/SJS",
        "เริ่ม lamotrigine ขนาดต่ำและเพิ่มช้าๆ ตาม label และให้คำแนะนำเรื่องผื่น",
        "Valproate inhibits lamotrigine glucuronidation",
        "Lamotrigine label; starter rule",
        DrugSelector::keys(&["valproate"]),
        DrugSelector::keys(&["lamotrigine"]),
    );
    ddi(
        &mut rules,
        "ddi.rifampicin.contraceptive",
        Severity::High,
        "rifampicin + oral contraceptive",
        "rifampicin ลดระดับฮอร์โมนคุมกำเนิดและอาจทำให้คุมกำเนิดล้มเหลว",
        "แนะนำวิธีคุมกำเนิดสำรองระหว่างและหลังใช้ rifampicin ตามคำแนะนำ",
        "CYP3A4 induction",
        "Rifampicin/contraceptive labels; starter rule",
        DrugSelector::keys(&["rifampicin", "rifabutin"]),
        DrugSelector::classes(&["oral_contraceptive"]),
    );
    ddi(
        &mut rules,
        "ddi.isotretinoin.tetracycline",
        Severity::High,
        "isotretinoin + tetracycline",
        "เพิ่มความเสี่ยง intracranial hypertension",
        "หลีกเลี่ยงการใช้ร่วม และเลือกยาปฏิชีวนะทางเลือก",
        "Additive intracranial pressure effect",
        "Isotretinoin label; starter rule",
        DrugSelector::keys(&["isotretinoin", "acitretin"]),
        DrugSelector::classes(&["tetracycline"]),
    );
    ddi(
        &mut rules,
        "ddi.qt.combination",
        Severity::High,
        "ยาหลายตัวที่ prolong QT",
        "การใช้ยาที่ prolong QT ตั้งแต่ 2 ตัวขึ้นไปเพิ่มความเสี่ยง torsades de pointes",
        "ทบทวนความจำเป็นของยาแต่ละตัว ตรวจ potassium/magnesium และพิจารณา ECG",
        "Additive QT prolongation",
        "CredibleMeds/tertiary references; starter rule",
        DrugSelector::classes(&["qt_prolonging"]),
        DrugSelector::classes(&["qt_prolonging"]),
    );
    ddi(
        &mut rules,
        "ddi.domperidone.qt",
        Severity::Critical,
        "domperidone + macrolide/azole",
        "เพิ่มความเสี่ยง QT prolongation และหัวใจเต้นผิดจังหวะรุนแรง",
        "หลีกเลี่ยงการใช้ร่วม โดยเฉพาะในผู้ป่วยโรคหัวใจหรือใช้ยาตัวอื่นที่ prolong QT",
        "CYP3A4 inhibition + additive QT effect",
        "Domperidone label; starter rule",
        DrugSelector::keys(&["domperidone"]),
        DrugSelector::any(&["clarithromycin", "erythromycin"], &["azole"]),
    );

    // ---- Duplicate therapy / burden ---------------------------------------
    dup(
        &mut rules,
        "dup.nsaid",
        "nsaid_non_aspirin",
        "NSAIDs",
        Severity::Moderate,
        "สั่งยา NSAID มากกว่าหนึ่งชนิด",
        "หลีกเลี่ยงการใช้ NSAID สองชนิดร่วมกันเพราะเพิ่มผลข้างเคียงโดยไม่เพิ่มประโยชน์",
        "Therapeutic duplication",
        "NSAID label; starter rule",
    );
    dup(
        &mut rules,
        "dup.ssri",
        "ssri",
        "SSRI",
        Severity::Moderate,
        "สั่งยา SSRI มากกว่าหนึ่งชนิด",
        "ทบทวนว่าตั้งใจเปลี่ยนยา/หยุดยาหรือไม่ และหลีกเลี่ยง serotonin excess",
        "Therapeutic duplication",
        "SSRI guideline; starter rule",
    );
    dup(
        &mut rules,
        "dup.benzodiazepine",
        "benzodiazepine",
        "benzodiazepine",
        Severity::Moderate,
        "สั่งยา benzodiazepine มากกว่าหนึ่งชนิด",
        "หลีกเลี่ยงการใช้ร่วมเพราะเพิ่ม sedation/fall/respiratory depression",
        "Therapeutic duplication",
        "Beers 2023; starter rule",
    );
    dup(
        &mut rules,
        "dup.z_drug",
        "z_drug",
        "Z-drug",
        Severity::Moderate,
        "สั่งยา Z-drug มากกว่าหนึ่งชนิด",
        "หลีกเลี่ยงการใช้ร่วมและทบทวนข้อบ่งใช้การนอนไม่หลับ",
        "Therapeutic duplication",
        "Beers 2023; starter rule",
    );
    dup(
        &mut rules,
        "dup.ppi",
        "ppi",
        "PPI",
        Severity::Low,
        "สั่งยา PPI มากกว่าหนึ่งชนิด",
        "โดยทั่วไปไม่จำเป็นต้องใช้ PPI สองชนิดร่วมกัน เว้นแต่มีข้อบ่งใช้เฉพาะ",
        "Therapeutic duplication",
        "Starter rule",
    );
    dup(
        &mut rules,
        "dup.statin",
        "statin",
        "statin",
        Severity::Moderate,
        "สั่งยา statin มากกว่าหนึ่งชนิด",
        "เพิ่มความเสี่ยง myopathy โดยไม่เพิ่มประโยชน์",
        "Therapeutic duplication",
        "Statin label; starter rule",
    );
    dup(
        &mut rules,
        "dup.acei",
        "acei",
        "ACEI",
        Severity::Moderate,
        "สั่งยา ACEI มากกว่าหนึ่งชนิด",
        "หลีกเลี่ยงการใช้ ACEI สองชนิดร่วมกัน",
        "Therapeutic duplication",
        "Starter rule",
    );
    dup(
        &mut rules,
        "dup.arb",
        "arb",
        "ARB",
        Severity::Moderate,
        "สั่งยา ARB มากกว่าหนึ่งชนิด",
        "หลีกเลี่ยงการใช้ ARB สองชนิดร่วมกัน",
        "Therapeutic duplication",
        "Starter rule",
    );
    dup(
        &mut rules,
        "dup.opioid",
        "opioid",
        "opioid",
        Severity::Moderate,
        "สั่งยา opioid มากกว่าหนึ่งชนิด",
        "ทบทวนความจำเป็นและเพิ่มความระวัง respiratory depression",
        "Therapeutic duplication",
        "Opioid safety; starter rule",
    );
    dup(
        &mut rules,
        "dup.antipsychotic",
        "antipsychotic",
        "antipsychotic",
        Severity::Moderate,
        "สั่งยารักษาโรคจิตมากกว่าหนึ่งชนิด",
        "ทบทวนข้อบ่งใช้และความเสี่ยง metabolic/EPS/QT",
        "Therapeutic duplication",
        "Starter rule",
    );
    dup(
        &mut rules,
        "dup.sulfonylurea",
        "sulfonylurea",
        "sulfonylurea",
        Severity::Moderate,
        "สั่งยา sulfonylurea มากกว่าหนึ่งชนิด",
        "เพิ่มความเสี่ยง hypoglycemia โดยไม่จำเป็น",
        "Therapeutic duplication",
        "Starter rule",
    );
    dup(
        &mut rules,
        "dup.insulin",
        "insulin",
        "insulin",
        Severity::Moderate,
        "สั่งอินซูลินมากกว่าหนึ่งชนิด",
        "อาจเป็น regimen ที่ตั้งใจ; ยืนยันชนิดและเวลาออกฤทธิ์เพื่อป้องกัน hypoglycemia",
        "Therapeutic duplication / regimen review",
        "Starter rule",
    );
    burden(
        &mut rules,
        "burden.anticholinergic",
        "anticholinergic",
        "anticholinergic",
        2,
        Severity::Moderate,
        "ยาหลายตัวที่มีฤทธิ์ anticholinergic",
        "เพิ่มความเสี่ยงสับสน ปากแห้ง ปัสสาวะคั่ง ท้องผูก และหกล้ม โดยเฉพาะในผู้สูงอายุ",
        "Anticholinergic burden",
        "Beers 2023 / ACB scale; starter rule",
    );
    burden(
        &mut rules,
        "burden.sedative",
        "sedative",
        "sedative",
        2,
        Severity::Moderate,
        "ยาหลายตัวที่กดประสาท",
        "เพิ่มความเสี่ยง sedation, fall และ respiratory depression",
        "Sedative burden",
        "Beers 2023; starter rule",
    );

    // ---- Renal ------------------------------------------------------------
    renal(
        &mut rules,
        "renal.metformin.egfr30",
        "metformin",
        "eGFR < 30",
        30.0,
        false,
        Severity::Critical,
        "metformin กับ eGFR ต่ำกว่า 30",
        "เสี่ยง lactic acidosis; แนะนำให้หยุด metformin",
        "หยุด metformin และปรึกษาแพทย์เรื่องยาลดน้ำตาลทางเลือก",
        "Metformin label / KDIGO 2024; starter rule",
    );
    renal(
        &mut rules,
        "renal.metformin.egfr45",
        "metformin",
        "eGFR < 45",
        45.0,
        false,
        Severity::High,
        "metformin กับ eGFR ต่ำกว่า 45",
        "ควรลดขนาดและไม่เริ่มยาใหม่; ต้องติดตามไตอย่างใกล้ชิด",
        "ทบทวนขนาด metformin และติดตาม eGFR",
        "Metformin label / KDIGO 2024; starter rule",
    );
    renal(
        &mut rules,
        "renal.nsaid.egfr30",
        "nsaid",
        "eGFR < 30",
        30.0,
        false,
        Severity::High,
        "NSAID กับ eGFR ต่ำกว่า 30",
        "เสี่ยงไตเสื่อมเพิ่มขึ้นและอาจทำให้การทำงานของไตแย่ลง",
        "หลีกเลี่ยง NSAID และเลือกยาแก้ปวดทางเลือก",
        "KDIGO 2024; starter rule",
    );
    renal(
        &mut rules,
        "renal.nsaid.egfr60",
        "nsaid",
        "eGFR < 60",
        60.0,
        false,
        Severity::Moderate,
        "NSAID กับ eGFR ต่ำกว่า 60",
        "ควรใช้ด้วยความระมัดระวังและระยะสั้น",
        "ใช้ระยะสั้นที่สุด ติดตาม creatinine และหลีกเลี่ยงเมื่อขาดน้ำ",
        "KDIGO 2024; starter rule",
    );
    renal(
        &mut rules,
        "renal.methotrexate.crcl30",
        "methotrexate",
        "CrCl < 30",
        30.0,
        true,
        Severity::Critical,
        "methotrexate กับ CrCl ต่ำกว่า 30",
        "การกำจัดยาลดลงมากและเพิ่มพิษรุนแรง",
        "หลีกเลี่ยงหรือปรึกษาแพทย์เพื่อปรับขนาด/หยุดยา",
        "Methotrexate label; starter rule",
    );
    renal(
        &mut rules,
        "renal.colchicine.crcl30",
        "colchicine",
        "CrCl < 30",
        30.0,
        true,
        Severity::Critical,
        "colchicine กับ CrCl ต่ำกว่า 30",
        "เสี่ยง colchicine toxicity รุนแรง",
        "หลีกเลี่ยงหรือลดขนาดตาม label อย่างเคร่งครัด",
        "Colchicine label; starter rule",
    );
    renal(
        &mut rules,
        "renal.digoxin.crcl30",
        "digoxin",
        "CrCl < 30",
        30.0,
        true,
        Severity::High,
        "digoxin กับ CrCl ต่ำกว่า 30",
        "digoxin ถูกกำจัดทางไตและสะสมได้",
        "ลดขนาดและติดตามระดับ digoxin/ชีพจร",
        "Digoxin label; starter rule",
    );
    renal(
        &mut rules,
        "renal.enoxaparin.crcl30",
        "enoxaparin",
        "CrCl < 30",
        30.0,
        true,
        Severity::High,
        "enoxaparin กับ CrCl ต่ำกว่า 30",
        "อาจต้องปรับขนาดเพื่อป้องกันเลือดออก",
        "ปรับขนาดตามข้อบ่งใช้และติดตาม platelet/renal function",
        "Enoxaparin label; starter rule",
    );
    renal(
        &mut rules,
        "renal.doac.crcl30",
        "doac",
        "CrCl < 30",
        30.0,
        true,
        Severity::High,
        "DOAC กับ CrCl ต่ำกว่า 30",
        "ระดับยาเพิ่มขึ้นและเสี่ยงเลือดออก; บางตัวห้ามใช้",
        "ตรวจสอบ label ของ DOAC แต่ละชนิดและปรับ/เปลี่ยนยา",
        "DOAC labels; starter rule",
    );
    renal(
        &mut rules,
        "renal.doac.crcl50",
        "doac",
        "CrCl < 50",
        50.0,
        true,
        Severity::Moderate,
        "DOAC กับ CrCl ต่ำกว่า 50",
        "อาจต้องปรับขนาดตามข้อบ่งใช้และความเสี่ยง",
        "ตรวจสอบขนาดและติดตาม renal function",
        "DOAC labels; starter rule",
    );
    renal(
        &mut rules,
        "renal.gabapentinoid.crcl30",
        "gabapentinoid",
        "CrCl < 30",
        30.0,
        true,
        Severity::High,
        "gabapentin/pregabalin กับ CrCl ต่ำกว่า 30",
        "ยาสะสมและเพิ่ม sedation/ataxia",
        "ลดขนาดตาม renal dosing และติดตามอาการ",
        "Gabapentin/pregabalin labels; starter rule",
    );
    renal(
        &mut rules,
        "renal.spironolactone.egfr30",
        "ksparing_diuretic",
        "eGFR < 30",
        30.0,
        false,
        Severity::High,
        "spironolactone/eplerenone กับ eGFR ต่ำกว่า 30",
        "เสี่ยง hyperkalemia",
        "หลีกเลี่ยงหรือติดตาม potassium/creatinine อย่างใกล้ชิด",
        "Spironolactone label; starter rule",
    );
    renal(
        &mut rules,
        "renal.potassium.egfr30",
        "potassium_chloride",
        "eGFR < 30",
        30.0,
        false,
        Severity::High,
        "potassium supplement กับ eGFR ต่ำกว่า 30",
        "เสี่ยง hyperkalemia",
        "หลีกเลี่ยงการเสริม potassium โดยไม่จำเป็น",
        "Potassium label; starter rule",
    );
    renal(
        &mut rules,
        "renal.allopurinol.egfr30",
        "xanthine_oxidase_inhibitor",
        "eGFR < 30",
        30.0,
        false,
        Severity::Moderate,
        "allopurinol/febuxostat กับ eGFR ต่ำกว่า 30",
        "อาจต้องปรับขนาดและติดตามอาการแพ้",
        "ปรับขนาดตาม renal function และติดตาม rash",
        "Allopurinol label; starter rule",
    );
    renal(
        &mut rules,
        "renal.metoclopramide.crcl30",
        "metoclopramide",
        "CrCl < 30",
        30.0,
        true,
        Severity::Moderate,
        "metoclopramide กับ CrCl ต่ำกว่า 30",
        "เพิ่มความเสี่ยง EPS",
        "ลดขนาดหรือหลีกเลี่ยง",
        "Metoclopramide label; starter rule",
    );
    renal(
        &mut rules,
        "renal.lithium.crcl30",
        "lithium",
        "CrCl < 30",
        30.0,
        true,
        Severity::High,
        "lithium กับ CrCl ต่ำกว่า 30",
        "ระดับ lithium สูงขึ้นและเสี่ยงพิษ",
        "ติดตามระดับ lithium อย่างใกล้ชิดและปรับขนาด",
        "Lithium label; starter rule",
    );

    // ---- Max daily dose ---------------------------------------------------
    maxdose(
        &mut rules,
        "dose.paracetamol.4000",
        "paracetamol",
        4000.0,
        None,
        Severity::High,
        "paracetamol เกิน 4,000 mg/วัน",
        "เพิ่มความเสี่ยงตับอักเสบรุนแรง",
        "ลดขนาดยาลงและทบทวนแหล่ง paracetamol อื่นที่ผู้ป่วยอาจได้รับ",
        "Paracetamol label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.paracetamol.elderly.3000",
        "paracetamol",
        3000.0,
        Some(65),
        Severity::High,
        "paracetamol เกิน 3,000 mg/วันในผู้สูงอายุ",
        "ผู้สูงอายุและผู้มีโรคตับมีความเสี่ยงตับสูงขึ้น",
        "ลดขนาดยาและพิจารณาปัจจัยเสี่ยงตับ",
        "Geriatric dosing reference; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.ibuprofen.2400",
        "ibuprofen",
        2400.0,
        None,
        Severity::High,
        "ibuprofen เกิน 2,400 mg/วัน",
        "เพิ่มความเสี่ยง GI bleeding/AKI",
        "ลดขนาดยาหรือเปลี่ยนยาแก้ปวด",
        "Ibuprofen label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.diclofenac.150",
        "diclofenac",
        150.0,
        None,
        Severity::High,
        "diclofenac เกิน 150 mg/วัน",
        "เพิ่มความเสี่ยง GI/renal/cardiovascular",
        "ลดขนาดหรือเปลี่ยนยา",
        "Diclofenac label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.naproxen.1250",
        "naproxen",
        1250.0,
        None,
        Severity::High,
        "naproxen เกิน 1,250 mg/วัน",
        "เพิ่มความเสี่ยง GI bleeding",
        "ลดขนาดหรือเปลี่ยนยา",
        "Naproxen label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.mefenamic.2000",
        "mefenamic_acid",
        2000.0,
        None,
        Severity::High,
        "mefenamic acid เกิน 2,000 mg/วัน",
        "เพิ่มความเสี่ยงชักและ GI bleeding",
        "ลดขนาดหรือเปลี่ยนยา",
        "Mefenamic acid label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.tramadol.400",
        "tramadol",
        400.0,
        None,
        Severity::High,
        "tramadol เกิน 400 mg/วัน",
        "เพิ่มความเสี่ยง seizure/serotonin toxicity",
        "ลดขนาดและหลีกเลี่ยงการใช้ร่วมกับ serotonergic drugs",
        "Tramadol label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.citalopram.40",
        "citalopram",
        40.0,
        None,
        Severity::High,
        "citalopram เกิน 40 mg/วัน",
        "เพิ่มความเสี่ยง QT prolongation",
        "ลดขนาดและตรวจ ECG หากมีปัจจัยเสี่ยง",
        "Citalopram label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.citalopram.elderly.20",
        "citalopram",
        20.0,
        Some(65),
        Severity::High,
        "citalopram เกิน 20 mg/วันในผู้สูงอายุ",
        "เพิ่มความเสี่ยง QT prolongation",
        "ลดขนาดเหลือไม่เกิน 20 mg/วัน",
        "Citalopram label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.escitalopram.20",
        "escitalopram",
        20.0,
        None,
        Severity::High,
        "escitalopram เกิน 20 mg/วัน",
        "เพิ่มความเสี่ยง QT prolongation",
        "ลดขนาดและตรวจ ECG หากมีปัจจัยเสี่ยง",
        "Escitalopram label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.escitalopram.elderly.10",
        "escitalopram",
        10.0,
        Some(65),
        Severity::High,
        "escitalopram เกิน 10 mg/วันในผู้สูงอายุ",
        "เพิ่มความเสี่ยง QT prolongation",
        "ลดขนาดเหลือไม่เกิน 10 mg/วัน",
        "Escitalopram label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.sertraline.200",
        "sertraline",
        200.0,
        None,
        Severity::High,
        "sertraline เกิน 200 mg/วัน",
        "เกินขนาดสูงสุดที่แนะนำ",
        "ลดขนาดยา",
        "Sertraline label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.fluoxetine.80",
        "fluoxetine",
        80.0,
        None,
        Severity::High,
        "fluoxetine เกิน 80 mg/วัน",
        "เกินขนาดสูงสุดที่แนะนำ",
        "ลดขนาดยา",
        "Fluoxetine label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.amitriptyline.150",
        "amitriptyline",
        150.0,
        None,
        Severity::High,
        "amitriptyline เกิน 150 mg/วัน",
        "เพิ่มความเสี่ยง anticholinergic/cardiac",
        "ลดขนาดและประเมิน ECG ในผู้สูงอายุ",
        "Amitriptyline label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.metoclopramide.30",
        "metoclopramide",
        30.0,
        None,
        Severity::High,
        "metoclopramide เกิน 30 mg/วัน",
        "เพิ่มความเสี่ยง EPS/tardive dyskinesia",
        "ลดขนาดและจำกัดระยะเวลาใช้",
        "Metoclopramide label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.simvastatin.40",
        "simvastatin",
        40.0,
        None,
        Severity::High,
        "simvastatin เกิน 40 mg/วัน",
        "เพิ่มความเสี่ยง myopathy",
        "ลดขนาดหรือเปลี่ยน statin",
        "Simvastatin label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.atorvastatin.80",
        "atorvastatin",
        80.0,
        None,
        Severity::High,
        "atorvastatin เกิน 80 mg/วัน",
        "เกินขนาดสูงสุดที่แนะนำ",
        "ตรวจสอบขนาดยา",
        "Atorvastatin label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.rosuvastatin.40",
        "rosuvastatin",
        40.0,
        None,
        Severity::High,
        "rosuvastatin เกิน 40 mg/วัน",
        "เพิ่มความเสี่ยง myopathy",
        "ตรวจสอบขนาดยาและ renal function",
        "Rosuvastatin label; starter rule",
    );
    maxdose(
        &mut rules,
        "dose.digoxin.elderly.0.125",
        "digoxin",
        0.125,
        Some(65),
        Severity::High,
        "digoxin เกิน 0.125 mg/วันในผู้สูงอายุ",
        "เพิ่มความเสี่ยงพิษจาก digoxin",
        "ทบทวนขนาดและระดับยา",
        "Beers 2023; starter rule",
    );

    // ---- Geriatric --------------------------------------------------------
    geriatric(
        &mut rules,
        "geri.benzodiazepine",
        DrugSelector::classes(&["benzodiazepine"]),
        Severity::High,
        "benzodiazepine ในผู้สูงอายุ",
        "เพิ่มความเสี่ยง sedation, fall, fracture และ cognitive impairment",
        "หลีกเลี่ยงหรือใช้ขนาดต่ำสุดระยะสั้น และพิจารณาทางเลือกที่ไม่ใช่ยา",
        "Beers 2023; starter rule",
    );
    geriatric(
        &mut rules,
        "geri.z_drug",
        DrugSelector::classes(&["z_drug"]),
        Severity::High,
        "Z-drug ในผู้สูงอายุ",
        "เพิ่มความเสี่ยง fall/fracture และ delirium",
        "หลีกเลี่ยงและใช้มาตรการสุขอนามัยการนอน",
        "Beers 2023; starter rule",
    );
    geriatric(
        &mut rules,
        "geri.anticholinergic",
        DrugSelector::classes(&["anticholinergic"]),
        Severity::High,
        "ยาที่มีฤทธิ์ anticholinergic ในผู้สูงอายุ",
        "เพิ่มความเสี่ยงสับสน ปัสสาวะคั่ง ท้องผูก และหกล้ม",
        "หลีกเลี่ยงหรือเลือกยาที่มีฤทธิ์ต่ำกว่า",
        "Beers 2023; starter rule",
    );
    geriatric(
        &mut rules,
        "geri.sedating_antihistamine",
        DrugSelector::classes(&["sedating_antihistamine"]),
        Severity::High,
        "ยาแก้แพ้ชนิดกดประสาทในผู้สูงอายุ",
        "เพิ่มความเสี่ยง sedation/confusion/urinary retention",
        "ใช้ยาแก้แพ้ชนิดไม่ง่วงหรือมาตรการที่ไม่ใช้ยา",
        "Beers 2023; starter rule",
    );
    geriatric(
        &mut rules,
        "geri.nsaid",
        DrugSelector::classes(&["nsaid"]),
        Severity::Moderate,
        "NSAID ในผู้สูงอายุ",
        "เพิ่มความเสี่ยง GI bleeding, AKI และ heart failure",
        "ใช้ระยะสั้นที่สุดและพิจารณายาแก้ปวดทางเลือก",
        "Beers 2023; starter rule",
    );
    geriatric(
        &mut rules,
        "geri.glibenclamide",
        DrugSelector::keys(&["glibenclamide"]),
        Severity::High,
        "glibenclamide ในผู้สูงอายุ",
        "เสี่ยง hypoglycemia ยาวนาน",
        "เปลี่ยนเป็นยาลดน้ำตาลที่เสี่ยง hypoglycemia น้อยกว่า",
        "Beers 2023; starter rule",
    );
    geriatric(
        &mut rules,
        "geri.antipsychotic",
        DrugSelector::classes(&["antipsychotic"]),
        Severity::Moderate,
        "ยารักษาโรคจิตในผู้สูงอายุ",
        "เพิ่มความเสี่ยง fall, EPS, metabolic และ mortality ใน dementia",
        "ทบทวนข้อบ่งใช้และใช้ขนาดต่ำสุด",
        "Beers 2023; starter rule",
    );
    geriatric(
        &mut rules,
        "geri.metoclopramide",
        DrugSelector::keys(&["metoclopramide"]),
        Severity::Moderate,
        "metoclopramide ในผู้สูงอายุ",
        "เพิ่มความเสี่ยง EPS/tardive dyskinesia",
        "จำกัดระยะเวลาและพิจารณาทางเลือก",
        "Beers 2023; starter rule",
    );
    geriatric(
        &mut rules,
        "geri.amitriptyline",
        DrugSelector::keys(&["amitriptyline"]),
        Severity::High,
        "amitriptyline ในผู้สูงอายุ",
        "anticholinergic และ orthostatic hypotension สูง",
        "หลีกเลี่ยงและเลือกยากลุ่มอื่น",
        "Beers 2023; starter rule",
    );
    geriatric(
        &mut rules,
        "geri.oxybutynin",
        DrugSelector::keys(&["oxybutynin"]),
        Severity::Moderate,
        "oxybutynin ในผู้สูงอายุ",
        "anticholinergic burden และ cognitive effects",
        "พิจารณายากลุ่มอื่นหรือมาตรการ non-drug",
        "Beers 2023; starter rule",
    );
    geriatric(
        &mut rules,
        "geri.pethidine",
        DrugSelector::keys(&["pethidine"]),
        Severity::High,
        "pethidine ในผู้สูงอายุ",
        "เมตาบอไลต์ normeperidine เพิ่มความเสี่ยงชัก",
        "หลีกเลี่ยงและใช้ opioid ทางเลือก",
        "Beers 2023; starter rule",
    );

    // ---- Drug-disease -----------------------------------------------------
    disease(
        &mut rules,
        "disease.nsaid.ckd",
        DrugSelector::classes(&["nsaid"]),
        &["N18", "N19"],
        "CKD",
        Severity::High,
        "NSAID กับโรคไตเรื้อรัง",
        "NSAID ลด renal blood flow และอาจทำให้ไตแย่ลง",
        "หลีกเลี่ยง NSAID และเลือกยาแก้ปวดที่ปลอดภัยต่อไต",
        "KDIGO 2024; starter rule",
    );
    disease(
        &mut rules,
        "disease.nsaid.hf",
        DrugSelector::classes(&["nsaid"]),
        &["I50"],
        "heart failure",
        Severity::High,
        "NSAID กับภาวะหัวใจล้มเหลว",
        "NSAID ทำให้เกิดน้ำคั่งและกำเริบ heart failure",
        "หลีกเลี่ยง NSAID",
        "Heart failure guideline; starter rule",
    );
    disease(
        &mut rules,
        "disease.nsaid.pud",
        DrugSelector::classes(&["nsaid"]),
        &["K25", "K26", "K27", "K92.0", "K92.1", "K92.2"],
        "peptic ulcer / GI bleed",
        Severity::High,
        "NSAID กับแผลกระเพาะหรือประวัติเลือดออกทางเดินอาหาร",
        "เพิ่มความเสี่ยงเลือดออกซ้ำ",
        "หลีกเลี่ยง NSAID หรือใช้เฉพาะเมื่อจำเป็นพร้อม gastroprotection",
        "NSAID label; starter rule",
    );
    disease(
        &mut rules,
        "disease.nsaid.cirrhosis",
        DrugSelector::classes(&["nsaid"]),
        &["K70.3", "K74"],
        "cirrhosis",
        Severity::High,
        "NSAID กับโรคตับแข็ง",
        "เพิ่มความเสี่ยงไตวายและเลือดออกจาก varices",
        "หลีกเลี่ยง NSAID",
        "Cirrhosis guideline; starter rule",
    );
    disease(
        &mut rules,
        "disease.metformin.liver",
        DrugSelector::keys(&["metformin"]),
        &["K70", "K71", "K72", "K73", "K74", "K75", "K76"],
        "liver disease",
        Severity::Moderate,
        "metformin กับโรคตับ",
        "เพิ่มความเสี่ยง lactic acidosis ในโรคตับรุนแรง",
        "ทบทวนข้อบ่งใช้และติดตาม liver function",
        "Metformin label; starter rule",
    );
    disease(
        &mut rules,
        "disease.nonselective_beta.asthma",
        DrugSelector::classes(&["non_selective_beta_blocker"]),
        &["J45"],
        "asthma",
        Severity::High,
        "beta-blocker ชนิดไม่จำเพาะกับโรคหอบหืด",
        "อาจทำให้หลอดลมหดเกร็งและกำเริบหอบ",
        "หลีกเลี่ยงหรือใช้ beta-1 selective ด้วยความระมัดระวัง",
        "Asthma guideline; starter rule",
    );
    disease(
        &mut rules,
        "disease.nonselective_beta.copd",
        DrugSelector::classes(&["non_selective_beta_blocker"]),
        &["J44"],
        "COPD",
        Severity::Moderate,
        "beta-blocker ชนิดไม่จำเพาะกับ COPD",
        "อาจเพิ่ม bronchospasm; beta-1 selective อาจปลอดภัยกว่า",
        "ทบทวนชนิดยาและติดตามอาการหายใจ",
        "COPD guideline; starter rule",
    );
    disease(
        &mut rules,
        "disease.antipsychotic.parkinson",
        DrugSelector::classes(&["antipsychotic"]),
        &["G20"],
        "Parkinson disease",
        Severity::High,
        "ยารักษาโรคจิตกับ Parkinson disease",
        "เพิ่มความเสี่ยง worsening motor symptoms",
        "หลีกเลี่ยงหรือใช้ quetiapine/clozapine ตามความเหมาะสมหลังปรึกษาแพทย์",
        "Parkinson guideline; starter rule",
    );
    disease(
        &mut rules,
        "disease.antipsychotic.dementia",
        DrugSelector::classes(&["antipsychotic"]),
        &["F00", "F01", "F02", "F03", "G30"],
        "dementia",
        Severity::Moderate,
        "ยารักษาโรคจิตกับ dementia",
        "เพิ่มความเสี่ยง mortality, stroke, fall",
        "ทบทวนข้อบ่งใช้และใช้ขนาดต่ำสุดระยะสั้น",
        "Beers 2023; starter rule",
    );
    disease(
        &mut rules,
        "disease.anticholinergic.bph",
        DrugSelector::classes(&["anticholinergic"]),
        &["N40"],
        "BPH",
        Severity::High,
        "ยาที่มีฤทธิ์ anticholinergic กับ BPH",
        "เสี่ยงปัสสาวะคั่งเฉียบพลัน",
        "หลีกเลี่ยงหรือเลือกยาที่มีฤทธิ์ต่ำกว่า",
        "Beers 2023; starter rule",
    );
    disease(
        &mut rules,
        "disease.anticholinergic.glaucoma",
        DrugSelector::classes(&["anticholinergic"]),
        &["H40"],
        "glaucoma",
        Severity::High,
        "ยาที่มีฤทธิ์ anticholinergic กับ glaucoma",
        "อาจเพิ่มความดันในลูกตา",
        "หลีกเลี่ยงและปรึกษาจักษุแพทย์หากจำเป็น",
        "Beers 2023; starter rule",
    );
    disease(
        &mut rules,
        "disease.corticosteroid.diabetes",
        DrugSelector::classes(&["corticosteroid"]),
        &["E10", "E11", "E12", "E13", "E14"],
        "diabetes",
        Severity::Moderate,
        "คอร์ติโคสเตียรอยด์กับ diabetes",
        "อาจทำให้น้ำตาลเลือดสูงขึ้น",
        "ติดตามน้ำตาลเลือดและปรับยาลดน้ำตาลตามความเหมาะสม",
        "Corticosteroid label; starter rule",
    );

    // ---- Lab --------------------------------------------------------------
    lab(
        &mut rules,
        "lab.potassium.high.ras",
        DrugSelector::classes(&["acei", "arb"]),
        LabKind::Potassium,
        CompareOp::Gt,
        5.0,
        Severity::High,
        "potassium สูงกับ ACEI/ARB",
        "เสี่ยง hyperkalemia และหัวใจเต้นผิดจังหวะ",
        "ยืนยันผล potassium ซ้ำและพิจารณาหยุด/ลดยา",
        "ACEI/ARB label; starter rule",
    );
    lab(
        &mut rules,
        "lab.potassium.high.ksparing",
        DrugSelector::classes(&["ksparing_diuretic"]),
        LabKind::Potassium,
        CompareOp::Gt,
        5.0,
        Severity::High,
        "potassium สูงกับ potassium-sparing diuretic",
        "เสี่ยง hyperkalemia",
        "ยืนยันผลและพิจารณาหยุด/ลดยา",
        "Spironolactone label; starter rule",
    );
    lab(
        &mut rules,
        "lab.potassium.high.supplement",
        DrugSelector::keys(&["potassium_chloride"]),
        LabKind::Potassium,
        CompareOp::Gt,
        5.0,
        Severity::Critical,
        "potassium สูงกับ potassium supplement",
        "เสี่ยง hyperkalemia รุนแรง",
        "หยุด potassium supplement และติดต่อแพทย์",
        "Potassium label; starter rule",
    );
    lab(
        &mut rules,
        "lab.potassium.low.digoxin",
        DrugSelector::keys(&["digoxin"]),
        LabKind::Potassium,
        CompareOp::Lt,
        3.5,
        Severity::High,
        "potassium ต่ำกับ digoxin",
        "hypokalemia เพิ่มความเสี่ยงพิษจาก digoxin",
        "แก้ไข potassium และติดตามระดับ digoxin",
        "Digoxin label; starter rule",
    );
    lab(
        &mut rules,
        "lab.inr.high.warfarin",
        DrugSelector::keys(&["warfarin"]),
        LabKind::Inr,
        CompareOp::Gt,
        3.0,
        Severity::Critical,
        "INR สูงกับ warfarin",
        "เพิ่มความเสี่ยงเลือดออก",
        "ประเมินอาการเลือดออกและปรึกษาแพทย์เรื่องการปรับขนาด/antidote",
        "Warfarin label; starter rule",
    );
    lab(
        &mut rules,
        "lab.alt.high.statin",
        DrugSelector::classes(&["statin"]),
        LabKind::Alt,
        CompareOp::Gt,
        120.0,
        Severity::High,
        "ALT สูงกับ statin",
        "อาจบ่งชี้ตับอักเสบจากยา",
        "ยืนยันผล ALT ซ้ำและทบทวนการหยุด/เปลี่ยน statin",
        "Statin label; starter rule",
    );
    lab(
        &mut rules,
        "lab.ast.high.statin",
        DrugSelector::classes(&["statin"]),
        LabKind::Ast,
        CompareOp::Gt,
        120.0,
        Severity::High,
        "AST สูงกับ statin",
        "อาจบ่งชี้ตับอักเสบจากยา",
        "ยืนยันผล AST ซ้ำและทบทวนการหยุด/เปลี่ยน statin",
        "Statin label; starter rule",
    );

    // ---- Pregnancy --------------------------------------------------------
    pregnancy(
        &mut rules,
        "preg.acei_arb",
        DrugSelector::classes(&["acei", "arb"]),
        Severity::Critical,
        "ACEI/ARB ระหว่างตั้งครรภ์",
        "ยาอาจทำให้ fetal renal failure/oligohydramnios",
        "หยุดยาและปรึกษาแพทย์เพื่อยาลดความดันทางเลือก",
        "ACEI/ARB labels; starter rule",
    );
    pregnancy(
        &mut rules,
        "preg.warfarin",
        DrugSelector::keys(&["warfarin"]),
        Severity::Critical,
        "warfarin ระหว่างตั้งครรภ์",
        "เสี่ยง fetal warfarin syndrome/bleeding",
        "หยุดและปรึกษาแพทย์เพื่อยาต้านการแข็งตัวทางเลือก",
        "Warfarin label; starter rule",
    );
    pregnancy(
        &mut rules,
        "preg.doac",
        DrugSelector::classes(&["doac"]),
        Severity::Critical,
        "DOAC ระหว่างตั้งครรภ์",
        "ข้อมูลความปลอดภัยไม่เพียงพอและอาจเป็นอันตราย",
        "เปลี่ยนเป็นยาต้านการแข็งตัวที่เหมาะสมหลังปรึกษาแพทย์",
        "DOAC labels; starter rule",
    );
    pregnancy(
        &mut rules,
        "preg.methotrexate",
        DrugSelector::keys(&["methotrexate"]),
        Severity::Critical,
        "methotrexate ระหว่างตั้งครรภ์",
        "ยามี teratogenic effect รุนแรง",
        "ห้ามใช้และปรึกษาแพทย์ทันที",
        "Methotrexate label; starter rule",
    );
    pregnancy(
        &mut rules,
        "preg.retinoid",
        DrugSelector::classes(&["retinoid"]),
        Severity::Critical,
        "isotretinoin/acitretin ระหว่างตั้งครรภ์",
        "ยามี teratogenic effect รุนแรง",
        "ห้ามใช้และปรึกษาแพทย์",
        "Retinoid labels; starter rule",
    );
    pregnancy(
        &mut rules,
        "preg.statin",
        DrugSelector::classes(&["statin"]),
        Severity::High,
        "statin ระหว่างตั้งครรภ์",
        "โดยทั่วไปแนะนำหยุด statin ระหว่างตั้งครรภ์",
        "หยุดยาและปรึกษาแพทย์",
        "Statin labels; starter rule",
    );
    pregnancy(
        &mut rules,
        "preg.tetracycline",
        DrugSelector::classes(&["tetracycline"]),
        Severity::High,
        "tetracycline ระหว่างตั้งครรภ์",
        "เสี่ยงฟัน discoloration และผลต่อกระดูก",
        "เลือกยาปฏิชีวนะทางเลือก",
        "Tetracycline label; starter rule",
    );
    pregnancy(
        &mut rules,
        "preg.nsaid",
        DrugSelector::classes(&["nsaid"]),
        Severity::High,
        "NSAID ระหว่างตั้งครรภ์",
        "โดยเฉพาะไตรมาสที่ 3 เสี่ยง ductus arteriosus/oligohydramnios",
        "หลีกเลี่ยง NSAID และเลือกยาแก้ปวดที่เหมาะสม",
        "NSAID label; starter rule",
    );
    pregnancy(
        &mut rules,
        "preg.valproate",
        DrugSelector::keys(&["valproate"]),
        Severity::Critical,
        "valproate ระหว่างตั้งครรภ์",
        "เสี่ยง neural tube defects และความผิดปกติแต่กำเนิด",
        "หลีกเลี่ยงและปรึกษาแพทย์เรื่องยาต้านชักทางเลือก",
        "Valproate label; starter rule",
    );
    pregnancy(
        &mut rules,
        "preg.lithium",
        DrugSelector::keys(&["lithium"]),
        Severity::High,
        "lithium ระหว่างตั้งครรภ์",
        "มีความเสี่ยงต่อ cardiac malformation และ neonatal toxicity",
        "ปรึกษาแพทย์และติดตามระดับยา",
        "Lithium label; starter rule",
    );

    // ---- Pediatric --------------------------------------------------------
    pediatric(
        &mut rules,
        "peds.codeine",
        DrugSelector::keys(&["codeine"]),
        11,
        Severity::Critical,
        "codeine ในเด็กอายุต่ำกว่า 12 ปี",
        "เสี่ยง respiratory depression จาก rapid metabolizer",
        "ห้ามใช้และเลือกยาแก้ปวดทางเลือก",
        "FDA codeine warning; starter rule",
    );
    pediatric(
        &mut rules,
        "peds.tramadol",
        DrugSelector::keys(&["tramadol"]),
        11,
        Severity::Critical,
        "tramadol ในเด็กอายุต่ำกว่า 12 ปี",
        "เสี่ยง respiratory depression",
        "ห้ามใช้และเลือกยาแก้ปวดทางเลือก",
        "FDA tramadol warning; starter rule",
    );
    pediatric(
        &mut rules,
        "peds.aspirin",
        DrugSelector::keys(&["aspirin"]),
        17,
        Severity::High,
        "aspirin ในเด็ก/วัยรุ่น",
        "เสี่ยง Reye syndrome โดยเฉพาะเมื่อมีการติดเชื้อไวรัส",
        "หลีกเลี่ยง aspirin และใช้ยาลดไข้ทางเลือก",
        "Aspirin label; starter rule",
    );
    pediatric(
        &mut rules,
        "peds.tetracycline",
        DrugSelector::classes(&["tetracycline"]),
        7,
        Severity::High,
        "tetracycline ในเด็กอายุต่ำกว่า 8 ปี",
        "เสี่ยงฟัน discoloration และผลต่อกระดูก",
        "เลือกยาปฏิชีวนะทางเลือก",
        "Tetracycline label; starter rule",
    );
    pediatric(
        &mut rules,
        "peds.fluoroquinolone",
        DrugSelector::classes(&["fluoroquinolone"]),
        17,
        Severity::Moderate,
        "fluoroquinolone ในเด็ก/วัยรุ่น",
        "มีความเสี่ยงต่อ cartilage/tendon; ใช้เมื่อประโยชน์มากกว่าความเสี่ยง",
        "ยืนยันข้อบ่งใช้และปรึกษาแพทย์",
        "Fluoroquinolone label; starter rule",
    );
    pediatric(
        &mut rules,
        "peds.promethazine",
        DrugSelector::keys(&["promethazine"]),
        1,
        Severity::Critical,
        "promethazine ในเด็กอายุต่ำกว่า 2 ปี",
        "เสี่ยง respiratory depression รุนแรง",
        "ห้ามใช้",
        "Promethazine label; starter rule",
    );

    rules
}

#[allow(clippy::too_many_arguments)]
fn rule(
    id: &str,
    category: Category,
    severity: Severity,
    title: &str,
    detail: &str,
    recommendation: &str,
    evidence: &str,
    source: &str,
    trigger: Trigger,
) -> Rule {
    Rule {
        id: id.into(),
        category,
        severity,
        title: title.into(),
        detail: detail.into(),
        recommendation: recommendation.into(),
        evidence: evidence.into(),
        source: source.into(),
        trigger,
        enabled_by_default: true,
        requires_ack: severity.requires_ack(),
    }
}

#[allow(clippy::too_many_arguments)]
fn ddi(
    rules: &mut Vec<Rule>,
    id: &str,
    severity: Severity,
    title: &str,
    detail: &str,
    recommendation: &str,
    evidence: &str,
    source: &str,
    a: DrugSelector,
    b: DrugSelector,
) {
    rules.push(rule(
        id,
        Category::Interaction,
        severity,
        title,
        detail,
        recommendation,
        evidence,
        source,
        Trigger::DrugPair { a, b },
    ));
}

#[allow(clippy::too_many_arguments)]
fn dup(
    rules: &mut Vec<Rule>,
    id: &str,
    class_key: &str,
    class_label: &str,
    severity: Severity,
    title: &str,
    detail: &str,
    evidence: &str,
    source: &str,
) {
    rules.push(rule(
        id,
        Category::DuplicateTherapy,
        severity,
        title,
        detail,
        "ทบทวนว่าตั้งใจสั่งยาซ้ำ/เปลี่ยนยากลุ่มเดียวกันหรือไม่ และเก็บไว้เพียงรายการที่จำเป็น",
        evidence,
        source,
        Trigger::DuplicateClass {
            class_key: class_key.into(),
            class_label: class_label.into(),
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn burden(
    rules: &mut Vec<Rule>,
    id: &str,
    class_key: &str,
    class_label: &str,
    min_count: usize,
    severity: Severity,
    title: &str,
    detail: &str,
    evidence: &str,
    source: &str,
) {
    rules.push(rule(
        id,
        Category::Geriatric,
        severity,
        title,
        detail,
        "ทบทวนยาที่มีผลเสริมกันและลดจำนวนรายการที่จำเป็น",
        evidence,
        source,
        Trigger::ClassBurden {
            class_key: class_key.into(),
            class_label: class_label.into(),
            min_count,
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn renal(
    rules: &mut Vec<Rule>,
    id: &str,
    drug_key_or_class: &str,
    condition_label: &str,
    threshold: f64,
    crcl: bool,
    severity: Severity,
    title: &str,
    detail: &str,
    recommendation: &str,
    source: &str,
) {
    let selector = if crate::dictionary::all_profiles()
        .iter()
        .any(|profile| profile.key == drug_key_or_class)
    {
        DrugSelector::keys(&[drug_key_or_class])
    } else {
        DrugSelector::classes(&[drug_key_or_class])
    };
    let condition = if crcl {
        PatientCondition::CrclBelow { value: threshold }
    } else {
        PatientCondition::EgfrBelow { value: threshold }
    };
    rules.push(rule(
        id,
        Category::Renal,
        severity,
        title,
        &format!("{detail} (เกณฑ์: {condition_label})"),
        recommendation,
        "Renal dosing review; verify against the current product label and KDIGO guidance.",
        source,
        Trigger::SingleDrug {
            drug: selector,
            condition: Some(condition),
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn maxdose(
    rules: &mut Vec<Rule>,
    id: &str,
    drug_key: &str,
    max_mg_per_day: f64,
    age_min: Option<u8>,
    severity: Severity,
    title: &str,
    detail: &str,
    recommendation: &str,
    source: &str,
) {
    rules.push(rule(
        id,
        Category::Dose,
        severity,
        title,
        detail,
        recommendation,
        "Maximum daily dose from the product label; dose estimate is derived from strength × sig.",
        source,
        Trigger::MaxDailyDose {
            drug: DrugSelector::keys(&[drug_key]),
            max_mg_per_day,
            age_min,
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn geriatric(
    rules: &mut Vec<Rule>,
    id: &str,
    drug: DrugSelector,
    severity: Severity,
    title: &str,
    detail: &str,
    recommendation: &str,
    source: &str,
) {
    let age = 65;
    rules.push(rule(
        id,
        Category::Geriatric,
        severity,
        title,
        detail,
        recommendation,
        "Potentially inappropriate medication in older adults; individualize and document the indication.",
        source,
        Trigger::SingleDrug {
            drug,
            condition: Some(PatientCondition::AgeAtLeast { years: age }),
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn disease(
    rules: &mut Vec<Rule>,
    id: &str,
    drug: DrugSelector,
    prefixes: &[&str],
    label: &str,
    severity: Severity,
    title: &str,
    detail: &str,
    recommendation: &str,
    source: &str,
) {
    rules.push(rule(
        id,
        Category::Disease,
        severity,
        title,
        detail,
        recommendation,
        "Drug-disease interaction; confirm the diagnosis and current disease control before acting.",
        source,
        Trigger::SingleDrug {
            drug,
            condition: Some(PatientCondition::Diagnosis {
                prefixes: prefixes.iter().map(|s| (*s).to_string()).collect(),
                label: label.to_string(),
            }),
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn lab(
    rules: &mut Vec<Rule>,
    id: &str,
    drug: DrugSelector,
    lab: LabKind,
    op: CompareOp,
    threshold: f64,
    severity: Severity,
    title: &str,
    detail: &str,
    recommendation: &str,
    source: &str,
) {
    rules.push(rule(
        id,
        Category::Lab,
        severity,
        title,
        detail,
        recommendation,
        "Drug-lab interaction; confirm the result is current and repeat if needed.",
        source,
        Trigger::LabThreshold {
            drug,
            lab,
            op,
            threshold,
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn pregnancy(
    rules: &mut Vec<Rule>,
    id: &str,
    drug: DrugSelector,
    severity: Severity,
    title: &str,
    detail: &str,
    recommendation: &str,
    source: &str,
) {
    rules.push(rule(
        id,
        Category::Pregnancy,
        severity,
        title,
        detail,
        recommendation,
        "Pregnancy safety review; confirm gestational age and indication with the prescriber.",
        source,
        Trigger::SingleDrug {
            drug,
            condition: Some(PatientCondition::Pregnancy),
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn pediatric(
    rules: &mut Vec<Rule>,
    id: &str,
    drug: DrugSelector,
    max_age: u8,
    severity: Severity,
    title: &str,
    detail: &str,
    recommendation: &str,
    source: &str,
) {
    rules.push(rule(
        id,
        Category::Pediatric,
        severity,
        title,
        detail,
        recommendation,
        "Pediatric safety review; verify age, weight, and indication.",
        source,
        Trigger::SingleDrug {
            drug,
            condition: Some(PatientCondition::AgeAtMost { years: max_age }),
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn pack_has_unique_non_empty_rules() {
        let pack = default_rule_pack();
        assert!(pack.rules.len() > 80, "starter pack should be substantial");
        let mut ids = HashSet::new();
        for rule in &pack.rules {
            assert!(
                ids.insert(rule.id.clone()),
                "duplicate rule id: {}",
                rule.id
            );
            assert!(!rule.title.trim().is_empty());
            assert!(!rule.detail.trim().is_empty());
            assert!(!rule.recommendation.trim().is_empty());
            assert!(!rule.source.trim().is_empty());
        }
    }

    #[test]
    fn every_referenced_class_has_a_thai_label() {
        let pack = default_rule_pack();
        for rule in &pack.rules {
            let mut classes = Vec::new();
            match &rule.trigger {
                Trigger::DrugPair { a, b } | Trigger::DrugTriple { a, b, .. } => {
                    classes.extend(a.classes.iter().cloned());
                    classes.extend(b.classes.iter().cloned());
                    classes.extend(match &rule.trigger {
                        Trigger::DrugTriple { c, .. } => c.classes.clone(),
                        _ => Vec::new(),
                    });
                }
                Trigger::SingleDrug { drug, .. }
                | Trigger::LabThreshold { drug, .. }
                | Trigger::MaxDailyDose { drug, .. } => {
                    classes.extend(drug.classes.iter().cloned());
                }
                Trigger::DuplicateClass { class_key, .. }
                | Trigger::ClassBurden { class_key, .. } => classes.push(class_key.clone()),
                Trigger::Allergy { cross, .. } => classes.extend(cross.classes.iter().cloned()),
            }
            for class in classes {
                assert_ne!(
                    crate::dictionary::class_label(&class),
                    "กลุ่มยาอื่น",
                    "rule {} references unknown class {}",
                    rule.id,
                    class
                );
            }
        }
    }

    #[test]
    fn overrides_change_enabled_state_severity_and_threshold() {
        let pack = default_rule_pack();
        let mut settings = ReviewSettings::default();
        let rule = pack
            .rules
            .iter()
            .find(|rule| rule.id == "renal.metformin.egfr30")
            .unwrap();
        assert_eq!(rule.severity, Severity::Critical);
        settings.overrides.insert(
            rule.id.clone(),
            RuleOverride {
                enabled: false,
                severity: Some(Severity::Moderate),
                threshold: Some(25.0),
            },
        );
        let effective = pack.effective_rules(&settings);
        let changed = effective.iter().find(|r| r.id == rule.id).unwrap();
        assert!(!changed.enabled_by_default);
        assert_eq!(changed.severity, Severity::Moderate);
        assert_eq!(changed.threshold_value(), Some(25.0));
    }

    #[test]
    fn signoff_matches_pack_version() {
        let pack = default_rule_pack();
        let mut settings = ReviewSettings::default();
        assert!(!pack.info(&settings).signed_off);
        settings.signoff = Some(RulePackSignoff {
            pharmacist_name: "ภก. ทดสอบ".into(),
            license_no: "12345".into(),
            signed_on: NaiveDate::from_ymd_opt(2026, 11, 4).unwrap(),
            pack_version: pack.version.clone(),
            note: "trial".into(),
        });
        assert!(pack.info(&settings).signed_off);
        settings.signoff.as_mut().unwrap().pack_version = "old".into();
        assert!(!pack.info(&settings).signed_off);
    }
}
