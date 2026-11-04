//! Deterministic, explainable review engine.
//!
//! The engine never calls a database. It receives today's orders plus the
//! clinical context and applies the site's effective rule pack. Every finding
//! names the orders it concerns, the patient facts that triggered it, and a
//! source. The coverage report is as important as the findings: it says when
//! a check could not run because a lab, diagnosis, weight, or drug mapping was
//! missing.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{
    AllergyRecord, CategoryCoverage, ClinicalContext, CoverageReport, Finding, LabKind, LabValue,
    MissingData, OrderLine, Patient, PregnancyStatus, ReviewAnalysis, ReviewSummary, RiskLevel,
};
use crate::rules::{
    Category, CheckStatus, PatientCondition, ReviewSettings, Rule, RulePack, Severity, Trigger,
};
use crate::thai;

/// Run the review.
pub fn analyze(
    patient: &Patient,
    orders: &[OrderLine],
    clinical: &ClinicalContext,
    settings: &ReviewSettings,
    pack: &RulePack,
) -> ReviewAnalysis {
    let effective_rules = pack.effective_rules(settings);
    let enabled_rules: Vec<Rule> = effective_rules
        .into_iter()
        .filter(|rule| settings.rule_enabled(rule))
        .collect();

    let mut builder = CoverageBuilder::new(patient, orders, clinical, settings, &enabled_rules);
    let mut findings = Vec::new();

    findings.extend(direct_allergy_findings(orders, clinical, &mut builder));

    for rule in &enabled_rules {
        match &rule.trigger {
            Trigger::DrugPair { a, b } => {
                if let Some(lines) = pair_matches(orders, a, b) {
                    findings.push(rule_finding(rule, &lines, orders, Vec::new(), None));
                }
            }
            Trigger::DrugTriple { a, b, c } => {
                if let Some(lines) = triple_matches(orders, a, b, c) {
                    findings.push(rule_finding(rule, &lines, orders, Vec::new(), None));
                }
            }
            Trigger::DuplicateClass {
                class_key,
                class_label,
            } => {
                let matched = matching_class(orders, class_key);
                if matched.len() >= 2 {
                    let lines: Vec<String> = matched.iter().map(|o| o.line_id.clone()).collect();
                    let factor = format!(
                        "กลุ่ม {}: {}",
                        class_label,
                        matched
                            .iter()
                            .map(|o| o.drug_name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    findings.push(rule_finding(rule, &lines, orders, vec![factor], None));
                }
            }
            Trigger::ClassBurden {
                class_key,
                class_label,
                min_count,
            } => {
                if !age_at_least(patient, settings.elderly_age) {
                    continue;
                }
                let matched = matching_class(orders, class_key);
                if matched.len() >= *min_count {
                    let lines: Vec<String> = matched.iter().map(|o| o.line_id.clone()).collect();
                    let factor = format!(
                        "{} รายการในกลุ่ม {}: {}",
                        matched.len(),
                        class_label,
                        matched
                            .iter()
                            .map(|o| o.drug_name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    findings.push(rule_finding(rule, &lines, orders, vec![factor], None));
                }
            }
            Trigger::SingleDrug { drug, condition } => {
                let matched = matching_selector(orders, drug);
                if matched.is_empty() {
                    continue;
                }
                let outcome = match condition {
                    None => ConditionOutcome::Applies,
                    Some(condition) => {
                        condition_outcome(condition, patient, clinical, settings, rule.category)
                    }
                };
                match outcome {
                    ConditionOutcome::Applies => {
                        let factors = condition_factors(condition, patient, clinical, settings);
                        for order in matched {
                            findings.push(rule_finding(
                                rule,
                                std::slice::from_ref(&order.line_id),
                                orders,
                                factors.clone(),
                                None,
                            ));
                        }
                    }
                    ConditionOutcome::NotApplies => {}
                    ConditionOutcome::Gap(missing) => builder.add_missing(missing),
                }
            }
            Trigger::LabThreshold {
                drug,
                lab,
                op,
                threshold,
            } => {
                let matched = matching_selector(orders, drug);
                if matched.is_empty() {
                    continue;
                }
                match lab_value(clinical, *lab) {
                    Some(value) => {
                        if op.evaluate(value.value, *threshold) {
                            let factor = format!(
                                "{} {:.2} {} (เกณฑ์ {}{}) วันที่ {}",
                                lab.label_th(),
                                value.value,
                                if value.unit.is_empty() {
                                    lab.unit()
                                } else {
                                    &value.unit
                                },
                                op.symbol(),
                                thai::format_qty(*threshold),
                                thai::format_thai_date_short(value.measured_on)
                            );
                            for order in matched {
                                findings.push(rule_finding(
                                    rule,
                                    std::slice::from_ref(&order.line_id),
                                    orders,
                                    vec![factor.clone()],
                                    None,
                                ));
                            }
                        }
                    }
                    None => builder.add_missing(MissingData {
                        label: format!("ไม่มีผล {} ล่าสุด", lab.label_th()),
                        detail: format!(
                            "กฎ \"{}\" ต้องใช้ผล {} เพื่อประเมิน",
                            rule.title,
                            lab.label_th()
                        ),
                        affects: vec![Category::Lab],
                    }),
                }
            }
            Trigger::MaxDailyDose {
                drug,
                max_mg_per_day,
                age_min,
            } => {
                if let Some(age_min) = age_min {
                    match patient.age_years {
                        Some(age) if age >= u32::from(*age_min) => {}
                        Some(_) => continue,
                        None => {
                            builder.add_missing(MissingData {
                                label: "ไม่ทราบอายุ".into(),
                                detail: "กฎขนาดยาตามอายุไม่สามารถประเมินได้".into(),
                                affects: vec![Category::Dose],
                            });
                            continue;
                        }
                    }
                }
                for order in matching_selector(orders, drug) {
                    match order.daily_dose_mg {
                        Some(dose) if dose > *max_mg_per_day => {
                            let mut factors = vec![format!(
                                "ขนาดประมาณ {} mg/วัน (เกิน {} mg/วัน)",
                                thai::format_qty(dose),
                                thai::format_qty(*max_mg_per_day)
                            )];
                            if let Some(age) = patient.age_years {
                                factors.push(format!("อายุ {age} ปี"));
                            }
                            findings.push(rule_finding(
                                rule,
                                std::slice::from_ref(&order.line_id),
                                orders,
                                factors,
                                Some(order.dose_note.clone()),
                            ));
                        }
                        Some(_) => {}
                        None => {
                            builder.dose_unparsed.insert(order.drug_name.clone());
                        }
                    }
                }
            }
            Trigger::Allergy {
                allergen_terms,
                cross,
            } => {
                let matched_allergies: Vec<&AllergyRecord> = clinical
                    .allergies
                    .iter()
                    .filter(|allergy| {
                        let agent = normalize(&allergy.agent);
                        allergen_terms
                            .iter()
                            .any(|term| agent.contains(&normalize(term)))
                    })
                    .collect();
                if matched_allergies.is_empty() {
                    continue;
                }
                let matched_orders = matching_selector(orders, cross);
                if matched_orders.is_empty() {
                    continue;
                }
                let severe = matched_allergies
                    .iter()
                    .any(|allergy| is_severe_allergy(&allergy.symptom));
                let force_no_order = matched_allergies
                    .iter()
                    .any(|allergy| allergy.force_no_order);
                let no_alert = matched_allergies.iter().any(|allergy| allergy.no_alert);
                let mut severity = rule.severity;
                if severe || force_no_order {
                    severity = Severity::Critical;
                }
                let factors: Vec<String> = matched_allergies
                    .iter()
                    .map(|allergy| allergy_factor(allergy))
                    .collect();
                let lines: Vec<String> = matched_orders.iter().map(|o| o.line_id.clone()).collect();
                let data_note = if no_alert {
                    Some("พบประวัติที่ HOSxP ตั้งค่า no_alert=Y; โปรดตรวจสอบว่าเป็นการปิดแจ้งเตือนที่ตั้งใจ".into())
                } else {
                    Some("ประวัติแพ้ยาแบบกลุ่ม".into())
                };
                let mut finding = rule_finding(rule, &lines, orders, factors, data_note);
                finding.severity = severity;
                finding.requires_ack = severity.requires_ack();
                findings.push(finding);
            }
        }
    }

    if settings.show_high_alert && settings.category_enabled(Category::HighAlert) {
        findings.extend(high_alert_findings(orders));
    }

    builder.add_global_missing();
    dedupe_findings(&mut findings);
    findings.retain(|finding| finding.severity.rank() <= settings.min_severity.rank());
    sort_findings(&mut findings, settings);
    let summary = summarize(&findings);
    let coverage = builder.finish(&findings, &summary, pack, settings);
    ReviewAnalysis {
        summary,
        findings,
        coverage,
    }
}

fn rule_finding(
    rule: &Rule,
    line_ids: &[String],
    orders: &[OrderLine],
    patient_factors: Vec<String>,
    data_note: Option<String>,
) -> Finding {
    let mut ids: Vec<&str> = line_ids.iter().map(|s| s.as_str()).collect();
    ids.sort_unstable();
    let id = format!("{}:{}", rule.id, ids.join("+"));
    let drug_names = orders
        .iter()
        .filter(|order| ids.contains(&order.line_id.as_str()))
        .map(|order| order.drug_name.clone())
        .collect();
    Finding {
        id,
        rule_id: rule.id.clone(),
        category: rule.category,
        severity: rule.severity,
        title: rule.title.clone(),
        detail: rule.detail.clone(),
        recommendation: rule.recommendation.clone(),
        evidence: rule.evidence.clone(),
        source: rule.source.clone(),
        patient_factors,
        drug_line_ids: line_ids.to_vec(),
        drug_names,
        requires_ack: rule.requires_ack,
        data_note,
    }
}

fn direct_allergy_findings(
    orders: &[OrderLine],
    clinical: &ClinicalContext,
    builder: &mut CoverageBuilder<'_>,
) -> Vec<Finding> {
    if !clinical.allergy_data_available {
        return Vec::new();
    }
    let mut findings = Vec::new();
    for (index, allergy) in clinical.allergies.iter().enumerate() {
        let agent = normalize(&allergy.agent);
        let has_icode = !allergy.icode.trim().is_empty();
        if !has_icode && (agent.len() < 3 || is_negative_allergy(&agent)) {
            continue;
        }
        for order in orders {
            let matched = if has_icode {
                order.icode.trim() == allergy.icode.trim()
            } else {
                drug_matches_agent(order, &agent)
            };
            if matched {
                let severe = is_severe_allergy(&allergy.symptom);
                let severity = if severe || allergy.force_no_order {
                    Severity::Critical
                } else {
                    Severity::High
                };
                let agent_label = allergy_agent_label(allergy);
                let detail = if allergy.force_no_order {
                    format!(
                        "ผู้ป่วยมีประวัติแพ้ \"{}\" และ HOSxP ตั้งค่า force_no_order=Y; รายการนี้คือ \"{}\"",
                        agent_label, order.drug_name
                    )
                } else {
                    format!(
                        "ผู้ป่วยมีประวัติแพ้ \"{}\" และรายการนี้คือ \"{}\"",
                        agent_label, order.drug_name
                    )
                };
                let recommendation = if allergy.force_no_order {
                    "HOSxP ตั้งค่า force_no_order=Y สำหรับประวัตินี้: อย่าจ่ายรายการนี้และติดต่อแพทย์ผู้สั่งยาเพื่อเลือกทางเลือกที่ปลอดภัย"
                        .into()
                } else if severe {
                    "แจ้งแพทย์ผู้สั่งยาและหลีกเลี่ยงยานี้ ตรวจสอบทางเลือกที่ปลอดภัยก่อนจ่าย".into()
                } else {
                    "ยืนยันประวัติการแพ้และอาการเดิมกับผู้ป่วย/แพทย์ก่อนจ่าย".into()
                };
                let data_note = if allergy.no_alert {
                    Some(
                        "HOSxP ตั้งค่า no_alert=Y สำหรับประวัตินี้; Drug-Ya ยังแสดงเพื่อให้เภสัชกรตรวจสอบ"
                            .into(),
                    )
                } else if has_icode {
                    Some(format!("จับคู่ด้วย opd_allergy.icode = {}", allergy.icode))
                } else {
                    None
                };
                findings.push(Finding {
                    id: format!("allergy.direct:{}:{index}", order.line_id),
                    rule_id: "allergy.direct".into(),
                    category: Category::Allergy,
                    severity,
                    title: "ประวัติแพ้ยาที่ตรงกับรายการสั่ง".into(),
                    detail,
                    recommendation,
                    evidence: if has_icode {
                        "Structured match between opd_allergy.icode and opitemrece.icode.".into()
                    } else {
                        "Direct match between opd_allergy.agent and the prescribed drug name/generic."
                            .into()
                    },
                    source: "HOSxP opd_allergy; pharmacist verification required".into(),
                    patient_factors: vec![allergy_factor(allergy)],
                    drug_line_ids: vec![order.line_id.clone()],
                    drug_names: vec![order.drug_name.clone()],
                    requires_ack: true,
                    data_note,
                });
            }
        }
    }
    if findings.is_empty() && clinical.allergies.is_empty() {
        builder.notes.push("ไม่พบประวัติแพ้ยาใน HOSxP".into());
    }
    findings
}

fn allergy_agent_label(allergy: &AllergyRecord) -> &str {
    if allergy.agent.trim().is_empty() {
        "ไม่ระบุชื่อยา"
    } else {
        allergy.agent.trim()
    }
}

fn allergy_factor(allergy: &AllergyRecord) -> String {
    let mut factor = format!("แพ้: {}", allergy_agent_label(allergy));
    if !allergy.symptom.trim().is_empty() {
        factor.push_str(&format!(" ({})", allergy.symptom.trim()));
    }
    if let Some(date) = allergy.report_date {
        factor.push_str(&format!(" วันที่ {}", thai::format_thai_date_short(date)));
    }
    if !allergy.icode.trim().is_empty() {
        factor.push_str(&format!(" · icode {}", allergy.icode.trim()));
    }
    if !allergy.seriousness_id.trim().is_empty() {
        factor.push_str(&format!(" · seriousness {}", allergy.seriousness_id.trim()));
    }
    if allergy.force_no_order {
        factor.push_str(" · force_no_order=Y");
    }
    if allergy.no_alert {
        factor.push_str(" · no_alert=Y");
    }
    factor
}

fn high_alert_findings(orders: &[OrderLine]) -> Vec<Finding> {
    orders
        .iter()
        .filter(|order| order.high_alert)
        .map(|order| Finding {
            id: format!("high_alert:{}", order.line_id),
            rule_id: "high_alert.generic".into(),
            category: Category::HighAlert,
            severity: Severity::Info,
            title: "ยาที่ต้องตรวจสอบซ้ำ (high-alert)".into(),
            detail: format!(
                "{} อยู่ในบัญชียาที่ต้องตรวจสอบซ้ำก่อนจ่าย",
                order.drug_name
            ),
            recommendation:
                "ตรวจสอบขนาด วิธีใช้ การคำนวณ และให้เภสัชกรอีกท่าน double-check ตามนโยบายของหน่วยงาน"
                    .into(),
            evidence: "High-alert medication requires an independent double-check.".into(),
            source: "ISMP high-alert medication list (site list may differ)".into(),
            patient_factors: vec![format!("รายการ: {}", order.drug_name)],
            drug_line_ids: vec![order.line_id.clone()],
            drug_names: vec![order.drug_name.clone()],
            requires_ack: true,
            data_note: None,
        })
        .collect()
}

fn pair_matches(
    orders: &[OrderLine],
    a: &crate::rules::DrugSelector,
    b: &crate::rules::DrugSelector,
) -> Option<Vec<String>> {
    let mut lines = BTreeSet::new();
    for i in 0..orders.len() {
        for j in (i + 1)..orders.len() {
            let first = &orders[i];
            let second = &orders[j];
            let matches = (a.matches(first.drug_key.as_deref(), &first.classes)
                && b.matches(second.drug_key.as_deref(), &second.classes))
                || (a.matches(second.drug_key.as_deref(), &second.classes)
                    && b.matches(first.drug_key.as_deref(), &first.classes));
            if matches {
                lines.insert(first.line_id.clone());
                lines.insert(second.line_id.clone());
            }
        }
    }
    (!lines.is_empty()).then(|| lines.into_iter().collect())
}

fn triple_matches(
    orders: &[OrderLine],
    a: &crate::rules::DrugSelector,
    b: &crate::rules::DrugSelector,
    c: &crate::rules::DrugSelector,
) -> Option<Vec<String>> {
    let mut lines = BTreeSet::new();
    for i in 0..orders.len() {
        for j in (i + 1)..orders.len() {
            for k in (j + 1)..orders.len() {
                let o = [&orders[i], &orders[j], &orders[k]];
                let permutations = [
                    [0usize, 1, 2],
                    [0, 2, 1],
                    [1, 0, 2],
                    [1, 2, 0],
                    [2, 0, 1],
                    [2, 1, 0],
                ];
                let matched = permutations.iter().any(|p| {
                    a.matches(o[p[0]].drug_key.as_deref(), &o[p[0]].classes)
                        && b.matches(o[p[1]].drug_key.as_deref(), &o[p[1]].classes)
                        && c.matches(o[p[2]].drug_key.as_deref(), &o[p[2]].classes)
                });
                if matched {
                    lines.insert(o[0].line_id.clone());
                    lines.insert(o[1].line_id.clone());
                    lines.insert(o[2].line_id.clone());
                }
            }
        }
    }
    (!lines.is_empty()).then(|| lines.into_iter().collect())
}

fn matching_selector<'a>(
    orders: &'a [OrderLine],
    selector: &crate::rules::DrugSelector,
) -> Vec<&'a OrderLine> {
    orders
        .iter()
        .filter(|order| selector.matches(order.drug_key.as_deref(), &order.classes))
        .collect()
}

fn matching_class<'a>(orders: &'a [OrderLine], class_key: &str) -> Vec<&'a OrderLine> {
    orders
        .iter()
        .filter(|order| order.classes.iter().any(|class| class == class_key))
        .collect()
}

enum ConditionOutcome {
    Applies,
    NotApplies,
    Gap(MissingData),
}

fn condition_outcome(
    condition: &PatientCondition,
    patient: &Patient,
    clinical: &ClinicalContext,
    settings: &ReviewSettings,
    category: Category,
) -> ConditionOutcome {
    match condition {
        PatientCondition::AgeAtLeast { years } => {
            let threshold = if category == Category::Geriatric {
                (*years).max(settings.elderly_age)
            } else {
                *years
            };
            match patient.age_years {
                Some(age) if age >= u32::from(threshold) => ConditionOutcome::Applies,
                Some(_) => ConditionOutcome::NotApplies,
                None => ConditionOutcome::Gap(MissingData {
                    label: "ไม่ทราบอายุ".into(),
                    detail: "กฎนี้ต้องใช้อายุผู้ป่วย".into(),
                    affects: vec![category],
                }),
            }
        }
        PatientCondition::AgeAtMost { years } => match patient.age_years {
            Some(age) if age <= u32::from(*years) => ConditionOutcome::Applies,
            Some(_) => ConditionOutcome::NotApplies,
            None => ConditionOutcome::Gap(MissingData {
                label: "ไม่ทราบอายุ".into(),
                detail: "กฎนี้ต้องใช้อายุผู้ป่วย".into(),
                affects: vec![category],
            }),
        },
        PatientCondition::EgfrBelow { value } => {
            let (reading, _source) = renal_reading(clinical, settings, false);
            match reading {
                Some(reading) if reading.value < *value => ConditionOutcome::Applies,
                Some(_) => ConditionOutcome::NotApplies,
                None => ConditionOutcome::Gap(MissingData {
                    label: "ไม่มีค่า eGFR/CrCl".into(),
                    detail: "กฎนี้ต้องใช้การทำงานของไตล่าสุด".into(),
                    affects: vec![Category::Renal],
                }),
            }
        }
        PatientCondition::CrclBelow { value } => {
            let (reading, _) = renal_reading(clinical, settings, true);
            match reading {
                Some(reading) if reading.value < *value => ConditionOutcome::Applies,
                Some(_) => ConditionOutcome::NotApplies,
                None => ConditionOutcome::Gap(MissingData {
                    label: "ไม่มีค่า CrCl/eGFR".into(),
                    detail: "กฎนี้ต้องใช้การทำงานของไตและน้ำหนักตัว".into(),
                    affects: vec![Category::Renal],
                }),
            }
        }
        PatientCondition::Diagnosis { prefixes, label } => {
            if !clinical.diagnosis_data_available {
                return ConditionOutcome::Gap(MissingData {
                    label: "อ่านรหัสวินิจฉัยไม่ได้".into(),
                    detail: "กฎนี้ต้องใช้รหัส ICD-10 ของผู้ป่วย".into(),
                    affects: vec![Category::Disease],
                });
            }
            if clinical.diagnoses.is_empty() {
                return ConditionOutcome::Gap(MissingData {
                    label: "ไม่พบรหัสวินิจฉัยใน 365 วัน".into(),
                    detail: format!("กฎ \"{label}\" ต้องใช้รหัส ICD-10"),
                    affects: vec![Category::Disease],
                });
            }
            if clinical
                .diagnoses
                .iter()
                .any(|diagnosis| icd_matches(&diagnosis.icd10, prefixes))
            {
                ConditionOutcome::Applies
            } else {
                ConditionOutcome::NotApplies
            }
        }
        PatientCondition::Pregnancy => match clinical.pregnancy {
            PregnancyStatus::Pregnant => ConditionOutcome::Applies,
            PregnancyStatus::NotPregnant | PregnancyStatus::NotApplicable => {
                ConditionOutcome::NotApplies
            }
            PregnancyStatus::Unknown => ConditionOutcome::Gap(MissingData {
                label: "ไม่ทราบสถานะตั้งครรภ์".into(),
                detail: "กฎการตั้งครรภ์ถูกข้ามเพื่อไม่ให้ผลลวง".into(),
                affects: vec![Category::Pregnancy],
            }),
        },
    }
}

fn condition_factors(
    condition: &Option<PatientCondition>,
    patient: &Patient,
    clinical: &ClinicalContext,
    settings: &ReviewSettings,
) -> Vec<String> {
    let Some(condition) = condition else {
        return Vec::new();
    };
    let mut factors = Vec::new();
    match condition {
        PatientCondition::AgeAtLeast { .. } | PatientCondition::AgeAtMost { .. } => {
            if let Some(age) = patient.age_years {
                factors.push(format!("อายุ {age} ปี"));
            }
        }
        PatientCondition::EgfrBelow { value } => {
            let (reading, source) = renal_reading(clinical, settings, false);
            if let Some(reading) = reading {
                factors.push(format!(
                    "{} {:.0} mL/min (เกณฑ์ <{})",
                    source,
                    reading.value,
                    thai::format_qty(*value)
                ));
            }
        }
        PatientCondition::CrclBelow { value } => {
            let (reading, source) = renal_reading(clinical, settings, true);
            if let Some(reading) = reading {
                factors.push(format!(
                    "{} {:.0} mL/min (เกณฑ์ <{})",
                    source,
                    reading.value,
                    thai::format_qty(*value)
                ));
            }
        }
        PatientCondition::Diagnosis { label, .. } => {
            if let Some(diagnosis) = clinical.diagnoses.iter().find(|d| {
                if let PatientCondition::Diagnosis { prefixes, .. } = condition {
                    icd_matches(&d.icd10, prefixes)
                } else {
                    false
                }
            }) {
                factors.push(format!(
                    "ICD-10: {} ({}) วันที่ {}",
                    diagnosis.icd10,
                    label,
                    thai::format_thai_date_short(diagnosis.date)
                ));
            }
        }
        PatientCondition::Pregnancy => {
            factors.push(format!(
                "สถานะตั้งครรภ์: {} ({})",
                clinical.pregnancy.label_th(),
                clinical.pregnancy_source
            ));
        }
    }
    factors
}

struct RenalReading {
    value: f64,
}

fn renal_reading(
    clinical: &ClinicalContext,
    settings: &ReviewSettings,
    intended_crcl: bool,
) -> (Option<RenalReading>, &'static str) {
    use crate::rules::RenalMetric;
    let prefer_crcl = match settings.renal_metric {
        RenalMetric::CockcroftGault => true,
        RenalMetric::CkdEpi => false,
        RenalMetric::Auto => intended_crcl,
    };
    if prefer_crcl {
        if let Some(value) = clinical.crcl {
            return (Some(RenalReading { value }), "CrCl");
        }
        if let Some(value) = clinical.egfr {
            return (Some(RenalReading { value }), "eGFR");
        }
    } else {
        if let Some(value) = clinical.egfr {
            return (Some(RenalReading { value }), "eGFR");
        }
        if let Some(value) = clinical.crcl {
            return (Some(RenalReading { value }), "CrCl");
        }
    }
    (None, "")
}

fn icd_matches(code: &str, prefixes: &[String]) -> bool {
    let code = code.trim().to_ascii_uppercase();
    prefixes
        .iter()
        .any(|prefix| code.starts_with(&prefix.to_ascii_uppercase()))
}

fn lab_value(clinical: &ClinicalContext, kind: LabKind) -> Option<&LabValue> {
    match kind {
        LabKind::Creatinine => clinical.creatinine.as_ref(),
        LabKind::Potassium => clinical.potassium.as_ref(),
        LabKind::Inr => clinical.inr.as_ref(),
        LabKind::Alt => clinical.alt.as_ref(),
        LabKind::Ast => clinical.ast.as_ref(),
        LabKind::Hba1c => clinical.hba1c.as_ref(),
        LabKind::Egfr => None,
    }
}

fn normalize(text: &str) -> String {
    text.chars()
        .flat_map(|c| c.to_lowercase())
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn is_negative_allergy(agent: &str) -> bool {
    [
        "nkda",
        "noknowndrugallergy",
        "noallergy",
        "none",
        "ไม่แพ้",
        "ไม่มีประวัติแพ้",
        "ไม่มียาแพ้",
    ]
    .iter()
    .any(|needle| agent.contains(needle))
}

fn is_severe_allergy(symptom: &str) -> bool {
    let text = symptom.to_lowercase();
    [
        "anaphyla",
        "angioedema",
        "airway",
        "stevens",
        "sjs",
        "ten",
        "toxic epidermal",
        "hypotension",
        "shock",
        "dyspnea",
        "หอบ",
        "หายใจไม่ออก",
        "บวม",
        "ช็อก",
        "ผื่นรุนแรง",
        "ผิวหนังลอก",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn drug_matches_agent(order: &OrderLine, agent: &str) -> bool {
    let mut candidates: Vec<String> = vec![normalize(&order.drug_name)];
    if let Some(generic) = &order.generic_name {
        candidates.push(normalize(generic));
    }
    if let Some(key) = &order.drug_key {
        candidates.push(normalize(&key.replace('_', " ")));
    }
    candidates.iter().any(|candidate| {
        candidate.len() >= 3
            && (candidate.contains(agent) || agent.contains(candidate))
            && !is_negative_allergy(candidate)
    })
}

fn age_at_least(patient: &Patient, threshold: u8) -> bool {
    patient
        .age_years
        .is_some_and(|age| age >= u32::from(threshold))
}

fn sort_findings(findings: &mut [Finding], settings: &ReviewSettings) {
    findings.sort_by(|a, b| {
        let by_severity = a.severity.rank().cmp(&b.severity.rank());
        let by_category = a.category.code().cmp(b.category.code());
        let by_title = a.title.cmp(&b.title);
        match settings.finding_sort {
            crate::rules::FindingSort::Severity => by_severity
                .then(by_category)
                .then(by_title)
                .then(a.id.cmp(&b.id)),
            crate::rules::FindingSort::Category => by_category
                .then(by_severity)
                .then(by_title)
                .then(a.id.cmp(&b.id)),
        }
    });
}

fn dedupe_findings(findings: &mut Vec<Finding>) {
    let mut kept: Vec<Finding> = Vec::with_capacity(findings.len());
    for finding in findings.drain(..) {
        let key = (
            finding.category,
            finding
                .drug_line_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join("+"),
        );
        let only_for = matches!(
            finding.category,
            Category::Renal | Category::Allergy | Category::Dose
        );
        if !only_for {
            kept.push(finding);
            continue;
        }
        if let Some(existing) = kept.iter_mut().find(|existing| {
            let existing_key = (
                existing.category,
                existing
                    .drug_line_ids
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join("+"),
            );
            existing_key == key
        }) {
            let replace = match finding.category {
                Category::Dose => {
                    finding.patient_factors.len() > existing.patient_factors.len()
                        || finding.severity.rank() < existing.severity.rank()
                }
                _ => finding.severity.rank() < existing.severity.rank(),
            };
            if replace {
                *existing = finding;
            }
        } else {
            kept.push(finding);
        }
    }
    *findings = kept;
}

fn summarize(findings: &[Finding]) -> ReviewSummary {
    let mut critical = 0;
    let mut high = 0;
    let mut moderate = 0;
    let mut low = 0;
    let mut info = 0;
    for finding in findings {
        match finding.severity {
            Severity::Critical => critical += 1,
            Severity::High => high += 1,
            Severity::Moderate => moderate += 1,
            Severity::Low => low += 1,
            Severity::Info => info += 1,
        }
    }
    let actionable = critical + high + moderate + low;
    let risk_level = if critical > 0 || high > 0 {
        RiskLevel::Urgent
    } else if moderate > 0 {
        RiskLevel::Review
    } else if actionable > 0 || info > 0 {
        RiskLevel::Monitor
    } else {
        RiskLevel::Clear
    };
    let headline = if actionable > 0 {
        let mut parts = Vec::new();
        if critical > 0 {
            parts.push(format!("{critical} วิกฤต"));
        }
        if high > 0 {
            parts.push(format!("{high} เสี่ยงสูง"));
        }
        if moderate > 0 {
            parts.push(format!("{moderate} เฝ้าระวัง"));
        }
        if low > 0 {
            parts.push(format!("{low} เล็กน้อย"));
        }
        let action = if critical > 0 {
            "ติดต่อแพทย์ก่อนจ่าย"
        } else if high > 0 {
            "ต้องทบทวนก่อนจ่าย"
        } else {
            "ควรทบทวนและเฝ้าระวัง"
        };
        format!("พบ {} รายการ — {}", parts.join(" • "), action)
    } else if info > 0 {
        format!("ไม่พบข้อท้วงติงสำคัญ — มี {info} รายการ high-alert ให้ตรวจสอบซ้ำ")
    } else {
        "ไม่พบข้อท้วงติงจากกฎที่เปิดใช้".to_string()
    };
    ReviewSummary {
        critical,
        high,
        moderate,
        low,
        info,
        actionable,
        headline,
        risk_level,
    }
}

struct CoverageBuilder<'a> {
    patient: &'a Patient,
    orders: &'a [OrderLine],
    clinical: &'a ClinicalContext,
    settings: &'a ReviewSettings,
    enabled_rules: &'a [Rule],
    missing: Vec<MissingData>,
    notes: Vec<String>,
    dose_unparsed: BTreeSet<String>,
    unknown_drugs: BTreeSet<String>,
}

impl<'a> CoverageBuilder<'a> {
    fn new(
        patient: &'a Patient,
        orders: &'a [OrderLine],
        clinical: &'a ClinicalContext,
        settings: &'a ReviewSettings,
        enabled_rules: &'a [Rule],
    ) -> Self {
        Self {
            patient,
            orders,
            clinical,
            settings,
            enabled_rules,
            missing: Vec::new(),
            notes: Vec::new(),
            dose_unparsed: BTreeSet::new(),
            unknown_drugs: orders
                .iter()
                .filter(|order| !order.known)
                .map(|order| order.drug_name.clone())
                .collect(),
        }
    }

    fn add_missing(&mut self, missing: MissingData) {
        if !self
            .missing
            .iter()
            .any(|existing| existing.label == missing.label)
        {
            self.missing.push(missing);
        }
    }

    fn add_global_missing(&mut self) {
        if !self.clinical.allergy_data_available {
            self.add_missing(MissingData {
                label: "อ่านประวัติแพ้ยาไม่ได้".into(),
                detail: "ตาราง opd_allergy อาจไม่มีหรือคอลัมน์ไม่ครบ".into(),
                affects: vec![Category::Allergy],
            });
        }
        if !self.clinical.lab_data_available {
            self.add_missing(MissingData {
                label: "อ่านผลตรวจทางห้องปฏิบัติการไม่ได้".into(),
                detail: "ตาราง lab_head/lab_order/lab_items อาจไม่มีหรือคอลัมน์ไม่ครบ".into(),
                affects: vec![Category::Lab, Category::Renal],
            });
        }
        if !self.clinical.diagnosis_data_available {
            self.add_missing(MissingData {
                label: "อ่านรหัสวินิจฉัยไม่ได้".into(),
                detail: "ตาราง ovstdiag/ovst อาจไม่มีหรือคอลัมน์ไม่ครบ".into(),
                affects: vec![Category::Disease],
            });
        }
        if !self.clinical.vitals_data_available {
            self.add_missing(MissingData {
                label: "อ่านน้ำหนัก/ส่วนสูงไม่ได้".into(),
                detail: "ตาราง opdscreen.bw/height อาจไม่มีหรือคอลัมน์ไม่ครบ".into(),
                affects: vec![Category::Renal],
            });
        }
        if self.clinical.creatinine.is_none() && self.clinical.lab_data_available {
            self.add_missing(MissingData {
                label: "ไม่มีผล creatinine ล่าสุด".into(),
                detail: "ไม่สามารถคำนวณ eGFR/CrCl หรือตรวจกฎไตได้".into(),
                affects: vec![Category::Renal],
            });
        }
        if self.clinical.weight_kg.is_none() {
            self.add_missing(MissingData {
                label: "ไม่มีน้ำหนักตัวล่าสุด".into(),
                detail: "ไม่สามารถคำนวณ Cockcroft-Gault CrCl ได้".into(),
                affects: vec![Category::Renal],
            });
        }
        if self.clinical.potassium.is_none() && self.clinical.lab_data_available {
            self.add_missing(MissingData {
                label: "ไม่มีผล potassium ล่าสุด".into(),
                detail: "กฎ potassium กับ ACEI/ARB, diuretic, digoxin จะไม่ทำงาน".into(),
                affects: vec![Category::Lab],
            });
        }
        if self.clinical.inr.is_none() && self.clinical.lab_data_available {
            self.add_missing(MissingData {
                label: "ไม่มีผล INR ล่าสุด".into(),
                detail: "กฎ INR กับ warfarin จะไม่ทำงาน".into(),
                affects: vec![Category::Lab],
            });
        }
        if self.clinical.diagnoses.is_empty() && self.clinical.diagnosis_data_available {
            self.add_missing(MissingData {
                label: "ไม่พบรหัสวินิจฉัยใน 365 วัน".into(),
                detail: "กฎ drug-disease จะตรวจได้เพียงบางส่วน".into(),
                affects: vec![Category::Disease],
            });
        }
        if self.settings.enable_pregnancy && self.clinical.pregnancy == PregnancyStatus::Unknown {
            self.add_missing(MissingData {
                label: "ไม่ทราบสถานะตั้งครรภ์".into(),
                detail: "กฎการตั้งครรภ์จะถูกข้ามจนกว่าจะยืนยันสถานะ".into(),
                affects: vec![Category::Pregnancy],
            });
        }
        if !self.dose_unparsed.is_empty() {
            let names = self
                .dose_unparsed
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            self.add_missing(MissingData {
                label: format!("ประมาณขนาดยารายวันไม่ได้ {} รายการ", self.dose_unparsed.len()),
                detail: format!("sig/ความแรงไม่ชัด: {names}"),
                affects: vec![Category::Dose],
            });
        }
        if !self.unknown_drugs.is_empty() {
            self.add_missing(MissingData {
                label: format!("ยังไม่รู้จักยา {} รายการ", self.unknown_drugs.len()),
                detail: "กฎ interaction/ซ้ำ/ไต/ขนาดยาอาจตรวจไม่ครบสำหรับรายการเหล่านี้".into(),
                affects: vec![
                    Category::Interaction,
                    Category::DuplicateTherapy,
                    Category::Renal,
                    Category::Dose,
                    Category::Disease,
                ],
            });
        }
        if self.orders.is_empty() {
            self.notes.push("ไม่พบรายการยาที่สั่งใน OPD วันนี้".into());
        } else {
            self.notes
                .push("ตรวจเฉพาะรายการยาที่สั่งใน OPD วันนี้ ไม่รวมยาที่ได้รับจากที่อื่น".into());
        }
        if self.settings.min_severity != Severity::Info {
            self.notes.push(format!(
                "ซ่อนรายการที่ความรุนแรงต่ำกว่า {}",
                self.settings.min_severity.label_th()
            ));
        }
    }

    fn finish(
        &self,
        findings: &[Finding],
        _summary: &ReviewSummary,
        pack: &RulePack,
        settings: &ReviewSettings,
    ) -> CoverageReport {
        let mut checked_by_category: BTreeMap<Category, usize> = BTreeMap::new();
        for rule in self.enabled_rules {
            *checked_by_category.entry(rule.category).or_insert(0) += 1;
        }
        let mut findings_by_category: BTreeMap<Category, usize> = BTreeMap::new();
        for finding in findings {
            *findings_by_category.entry(finding.category).or_insert(0) += 1;
        }
        let categories = Category::all()
            .iter()
            .map(|category| {
                let checked_rules = checked_by_category.get(category).copied().unwrap_or(0);
                let finding_count = findings_by_category.get(category).copied().unwrap_or(0);
                let (status, note) = self.category_status(*category, finding_count);
                CategoryCoverage {
                    category: *category,
                    status,
                    checked_rules,
                    findings: finding_count,
                    note,
                }
            })
            .collect();

        let mut unresolved_drugs: Vec<String> = self.unknown_drugs.iter().cloned().collect();
        unresolved_drugs.sort();
        let applied_rule_count = findings
            .iter()
            .map(|finding| finding.rule_id.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let mut notes = self.notes.clone();
        if !settings
            .signoff
            .as_ref()
            .is_some_and(|signoff| signoff.pack_version == pack.version)
        {
            notes.push(format!(
                "ชุดกฎ {} ยังไม่ลงนามรับรอง (ต้องลงนามในหน้าตั้งค่า)",
                pack.version
            ));
        }
        notes.sort();
        notes.dedup();
        CoverageReport {
            categories,
            missing_data: self.missing.clone(),
            unresolved_drugs,
            known_drug_count: self.orders.iter().filter(|o| o.known).count(),
            total_order_count: self.orders.len(),
            checked_rule_count: self.enabled_rules.len(),
            applied_rule_count,
            notes,
        }
    }

    fn category_status(&self, category: Category, finding_count: usize) -> (CheckStatus, String) {
        let findings = if finding_count > 0 {
            format!(" • พบ {finding_count} รายการ")
        } else {
            String::new()
        };
        match category {
            Category::Allergy => {
                if !self.clinical.allergy_data_available {
                    (CheckStatus::Skipped, "อ่านตารางแพ้ยาไม่ได้".into())
                } else if self.clinical.allergies.is_empty() {
                    (CheckStatus::Checked, "ไม่พบประวัติแพ้ยาใน HOSxP".into())
                } else {
                    (
                        CheckStatus::Checked,
                        format!(
                            "ตรวจประวัติแพ้ยา {} รายการ{}",
                            self.clinical.allergies.len(),
                            findings
                        ),
                    )
                }
            }
            Category::Interaction => {
                if self.orders.len() < 2 {
                    (CheckStatus::Skipped, "ต้องมีรายการยาอย่างน้อย 2 รายการ".into())
                } else if !self.unknown_drugs.is_empty() {
                    (
                        CheckStatus::Partial,
                        format!(
                            "มียาที่ไม่รู้จัก {} รายการ อาจตรวจไม่ครบ{}",
                            self.unknown_drugs.len(),
                            findings
                        ),
                    )
                } else {
                    (
                        CheckStatus::Checked,
                        format!("ตรวจคู่ยาทั้งหมด {} รายการ{}", self.orders.len(), findings),
                    )
                }
            }
            Category::DuplicateTherapy => {
                if self.orders.len() < 2 {
                    (CheckStatus::Skipped, "ต้องมีรายการยาอย่างน้อย 2 รายการ".into())
                } else if !self.unknown_drugs.is_empty() {
                    (
                        CheckStatus::Partial,
                        format!("มียาที่ไม่รู้จัก {} รายการ{}", self.unknown_drugs.len(), findings),
                    )
                } else {
                    (CheckStatus::Checked, format!("ตรวจกลุ่มยาซ้ำแล้ว{findings}"))
                }
            }
            Category::Renal => {
                if self.clinical.egfr.is_none() && self.clinical.crcl.is_none() {
                    (
                        CheckStatus::Skipped,
                        "ไม่มี creatinine/น้ำหนักที่ใช้คำนวณไต".into(),
                    )
                } else if self.clinical.weight_kg.is_none() {
                    (
                        CheckStatus::Partial,
                        format!("ใช้ eGFR แต่ไม่มีน้ำหนักสำหรับ CrCl{findings}"),
                    )
                } else if !self.unknown_drugs.is_empty() {
                    (
                        CheckStatus::Partial,
                        format!("มียาที่ไม่รู้จัก {} รายการ{findings}", self.unknown_drugs.len()),
                    )
                } else {
                    (
                        CheckStatus::Checked,
                        format!("ตรวจขนาดยาตามการทำงานของไต{findings}"),
                    )
                }
            }
            Category::Dose => {
                if self.orders.is_empty() {
                    (CheckStatus::Skipped, "ไม่มีรายการยา".into())
                } else if !self.dose_unparsed.is_empty() {
                    (
                        CheckStatus::Partial,
                        format!(
                            "อ่านขนาดยาไม่ได้ {} รายการ{findings}",
                            self.dose_unparsed.len()
                        ),
                    )
                } else {
                    (
                        CheckStatus::Checked,
                        format!("ประเมินขนาดยาจาก sig{findings}"),
                    )
                }
            }
            Category::Geriatric => match self.patient.age_years {
                None => (CheckStatus::Skipped, "ไม่ทราบอายุ".into()),
                Some(age) if age < u32::from(self.settings.elderly_age) => (
                    CheckStatus::Skipped,
                    format!("อายุ {age} ปี ต่ำกว่าเกณฑ์ {} ปี", self.settings.elderly_age),
                ),
                Some(age) => (CheckStatus::Checked, format!("อายุ {age} ปี{findings}")),
            },
            Category::Disease => {
                if !self.clinical.diagnosis_data_available {
                    (CheckStatus::Skipped, "อ่านรหัสวินิจฉัยไม่ได้".into())
                } else if self.clinical.diagnoses.is_empty() {
                    (CheckStatus::Partial, "ไม่พบรหัส ICD-10 ใน 365 วัน".into())
                } else if !self.unknown_drugs.is_empty() {
                    (
                        CheckStatus::Partial,
                        format!(
                            "ตรวจ {} รหัสวินิจฉัย • มียาที่ไม่รู้จัก{}",
                            self.clinical.diagnoses.len(),
                            findings
                        ),
                    )
                } else {
                    (
                        CheckStatus::Checked,
                        format!(
                            "ตรวจ {} รหัสวินิจฉัย{}",
                            self.clinical.diagnoses.len(),
                            findings
                        ),
                    )
                }
            }
            Category::Lab => {
                if !self.clinical.lab_data_available {
                    (CheckStatus::Skipped, "อ่านผลตรวจไม่ได้".into())
                } else if self.clinical.potassium.is_none()
                    && self.clinical.inr.is_none()
                    && self.clinical.alt.is_none()
                    && self.clinical.ast.is_none()
                {
                    (CheckStatus::Partial, "ไม่พบผล lab ที่กฎต้องใช้".into())
                } else {
                    (CheckStatus::Checked, format!("ตรวจผล lab ล่าสุด{findings}"))
                }
            }
            Category::Pregnancy => {
                if !self.settings.enable_pregnancy {
                    (CheckStatus::Skipped, "ปิดการตรวจการตั้งครรภ์".into())
                } else {
                    match self.clinical.pregnancy {
                        PregnancyStatus::Pregnant => (
                            CheckStatus::Checked,
                            format!("ตั้งครรภ์ ({}){findings}", self.clinical.pregnancy_source),
                        ),
                        PregnancyStatus::NotPregnant => (CheckStatus::Checked, "ไม่ตั้งครรภ์".into()),
                        PregnancyStatus::NotApplicable => {
                            (CheckStatus::Skipped, "ไม่เกี่ยวข้องกับผู้ป่วยรายนี้".into())
                        }
                        PregnancyStatus::Unknown => {
                            (CheckStatus::Partial, "ยังไม่ทราบสถานะตั้งครรภ์".into())
                        }
                    }
                }
            }
            Category::Pediatric => {
                if !self.settings.enable_pediatric {
                    (CheckStatus::Skipped, "ปิดการตรวจเด็ก".into())
                } else {
                    match self.patient.age_years {
                        None => (CheckStatus::Skipped, "ไม่ทราบอายุ".into()),
                        Some(age) if age >= 18 => {
                            (CheckStatus::Skipped, format!("อายุ {age} ปี ไม่อยู่ในเกณฑ์เด็ก"))
                        }
                        Some(age) => (CheckStatus::Checked, format!("อายุ {age} ปี{findings}")),
                    }
                }
            }
            Category::HighAlert => {
                if !self.settings.show_high_alert
                    || !self.settings.category_enabled(Category::HighAlert)
                {
                    (CheckStatus::Skipped, "ปิดรายการ high-alert".into())
                } else {
                    let count = self.orders.iter().filter(|o| o.high_alert).count();
                    (
                        CheckStatus::Checked,
                        format!("รายการ high-alert {count} รายการ{findings}"),
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::resolve_drug;
    use crate::dose::estimate_dose;
    use crate::model::{DiagnosisRecord, LabFlag, LabValue, OrderLine, Patient, Sex};
    use crate::rules::default_rule_pack;
    use chrono::NaiveDate;

    fn patient(age: u32, sex: Sex) -> Patient {
        Patient {
            hn: "0001234".into(),
            cid: "1101701234567".into(),
            full_name_th: "นายทดสอบ ใจดี".into(),
            birth_date: Some(NaiveDate::from_ymd_opt(2026 - age as i32, 1, 1).unwrap()),
            sex,
            age_years: Some(age),
        }
    }

    fn order(name: &str, strength: &str, units: &str, sig: &str, qty: f64, id: &str) -> OrderLine {
        let resolution = resolve_drug(name);
        let dose = estimate_dose(strength, units, sig, qty);
        OrderLine {
            line_id: id.into(),
            vn: "VN1".into(),
            icode: "I1".into(),
            drug_name: name.into(),
            strength: strength.into(),
            units: units.into(),
            sig: sig.into(),
            qty,
            item_no: 1,
            doctor_name: "นพ.ทดสอบ".into(),
            ordered_on: Some(NaiveDate::from_ymd_opt(2026, 11, 4).unwrap()),
            ordered_at: Some("09:00".into()),
            drug_key: resolution.key,
            generic_name: resolution.display_name,
            classes: resolution.classes,
            high_alert: resolution.high_alert,
            known: resolution.known,
            daily_dose_mg: dose.mg_per_day,
            days_supply: dose.days_supply,
            days_supply_note: dose.days_supply_note,
            dose_note: dose.note,
        }
    }

    fn clinical() -> ClinicalContext {
        ClinicalContext {
            age_years: Some(72),
            sex: Sex::Male,
            weight_kg: Some(70.0),
            height_cm: Some(170.0),
            bmi: Some(24.2),
            creatinine: Some(LabValue {
                value: 1.0,
                text: "1.0".into(),
                unit: "mg/dL".into(),
                measured_on: NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
                item_name: "Creatinine".into(),
                flag: LabFlag::Normal,
            }),
            egfr: Some(104.0),
            egfr_source: "CKD-EPI 2021".into(),
            crcl: Some(80.0),
            potassium: None,
            inr: None,
            alt: None,
            ast: None,
            hba1c: None,
            pregnancy: PregnancyStatus::NotApplicable,
            pregnancy_source: "male".into(),
            diagnoses: vec![],
            pmh: None,
            allergies: vec![],
            allergy_data_available: true,
            lab_data_available: true,
            diagnosis_data_available: true,
            vitals_data_available: true,
            medication_scope_note: "today".into(),
        }
    }

    #[test]
    fn warfarin_and_ibuprofen_produces_bleeding_interaction() {
        let orders = vec![
            order("Warfarin 3 mg", "3 mg", "Tab", "1x1", 30.0, "a"),
            order("Ibuprofen 400 mg", "400 mg", "Tab", "1x3", 30.0, "b"),
        ];
        let pack = default_rule_pack();
        let analysis = analyze(
            &patient(72, Sex::Male),
            &orders,
            &clinical(),
            &ReviewSettings::default(),
            &pack,
        );
        assert!(
            analysis
                .findings
                .iter()
                .any(|f| f.rule_id == "ddi.warfarin.nsaid" && f.severity == Severity::High),
            "{:#?}",
            analysis.findings
        );
        assert!(analysis.summary.high >= 1);
    }

    #[test]
    fn duplicate_nsaid_is_detected() {
        let orders = vec![
            order("Ibuprofen 400 mg", "400 mg", "Tab", "1x3", 30.0, "a"),
            order("Diclofenac 25 mg", "25 mg", "Tab", "1x3", 30.0, "b"),
        ];
        let analysis = analyze(
            &patient(40, Sex::Female),
            &orders,
            &clinical(),
            &ReviewSettings::default(),
            &default_rule_pack(),
        );
        assert!(analysis.findings.iter().any(|f| f.rule_id == "dup.nsaid"));
    }

    #[test]
    fn direct_allergy_match_is_critical_for_anaphylaxis() {
        let orders = vec![order(
            "Amoxicillin 500 mg",
            "500 mg",
            "Cap",
            "1x3",
            30.0,
            "a",
        )];
        let mut c = clinical();
        c.allergies = vec![AllergyRecord {
            agent: "Amoxicillin".into(),
            symptom: "anaphylaxis".into(),
            reporter: "doctor".into(),
            report_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            note: String::new(),
            icode: String::new(),
            seriousness_id: String::new(),
            no_alert: false,
            force_no_order: false,
        }];
        let analysis = analyze(
            &patient(40, Sex::Female),
            &orders,
            &c,
            &ReviewSettings::default(),
            &default_rule_pack(),
        );
        assert!(
            analysis
                .findings
                .iter()
                .any(|f| f.rule_id == "allergy.direct" && f.severity == Severity::Critical)
        );
    }

    #[test]
    fn structured_allergy_icode_and_force_no_order_are_honored() {
        let orders = vec![order(
            "Amoxicillin 500 mg",
            "500 mg",
            "Cap",
            "1x3",
            30.0,
            "a",
        )];
        let mut c = clinical();
        c.allergies = vec![AllergyRecord {
            agent: String::new(),
            symptom: "ผื่น".into(),
            reporter: String::new(),
            report_date: None,
            note: String::new(),
            icode: "I1".into(),
            seriousness_id: "4".into(),
            no_alert: true,
            force_no_order: true,
        }];
        let analysis = analyze(
            &patient(40, Sex::Female),
            &orders,
            &c,
            &ReviewSettings::default(),
            &default_rule_pack(),
        );
        let finding = analysis
            .findings
            .iter()
            .find(|f| f.rule_id == "allergy.direct")
            .expect("icode allergy match");
        assert_eq!(finding.severity, Severity::Critical);
        assert!(
            finding
                .patient_factors
                .iter()
                .any(|f| f.contains("icode I1"))
        );
        assert!(
            finding
                .data_note
                .as_deref()
                .is_some_and(|note| note.contains("no_alert"))
        );
    }

    #[test]
    fn metformin_is_critical_at_low_egfr() {
        let orders = vec![order("Metformin 500 mg", "500 mg", "Tab", "1x2", 60.0, "a")];
        let mut c = clinical();
        c.egfr = Some(25.0);
        c.crcl = Some(25.0);
        let analysis = analyze(
            &patient(70, Sex::Male),
            &orders,
            &c,
            &ReviewSettings::default(),
            &default_rule_pack(),
        );
        assert!(
            analysis
                .findings
                .iter()
                .any(|f| f.rule_id == "renal.metformin.egfr30" && f.severity == Severity::Critical),
            "{:#?}",
            analysis.findings
        );
    }

    #[test]
    fn unknown_drug_is_reported_in_coverage() {
        let orders = vec![order("Mystery Syrup", "", "Bottle", "1x3", 1.0, "a")];
        let analysis = analyze(
            &patient(40, Sex::Female),
            &orders,
            &clinical(),
            &ReviewSettings::default(),
            &default_rule_pack(),
        );
        assert_eq!(analysis.coverage.known_drug_count, 0);
        assert_eq!(analysis.coverage.unresolved_drugs.len(), 1);
    }

    #[test]
    fn pregnancy_rule_is_skipped_when_status_unknown() {
        let orders = vec![order("Enalapril 5 mg", "5 mg", "Tab", "1x1", 30.0, "a")];
        let mut c = clinical();
        c.pregnancy = PregnancyStatus::Unknown;
        c.pregnancy_source = "no data".into();
        let analysis = analyze(
            &patient(30, Sex::Female),
            &orders,
            &c,
            &ReviewSettings::default(),
            &default_rule_pack(),
        );
        assert!(
            !analysis
                .findings
                .iter()
                .any(|f| f.category == Category::Pregnancy)
        );
        assert!(
            analysis
                .coverage
                .missing_data
                .iter()
                .any(|m| m.label.contains("ตั้งครรภ์"))
        );
    }

    #[test]
    fn diagnosis_rule_matches_icd_prefix() {
        let orders = vec![order("Ibuprofen 400 mg", "400 mg", "Tab", "1x3", 30.0, "a")];
        let mut c = clinical();
        c.diagnoses = vec![DiagnosisRecord {
            icd10: "N18.3".into(),
            label: Some("CKD".into()),
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            diag_type: "1".into(),
        }];
        let analysis = analyze(
            &patient(60, Sex::Male),
            &orders,
            &c,
            &ReviewSettings::default(),
            &default_rule_pack(),
        );
        assert!(
            analysis
                .findings
                .iter()
                .any(|f| f.rule_id == "disease.nsaid.ckd")
        );
    }
}
