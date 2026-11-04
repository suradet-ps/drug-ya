//! Review workspace: a clean patient header, one clinical summary, one context
//! bar, today's orders, and the findings panel.
//!
//! The screen is intentionally quiet: primary facts stay visible, secondary
//! detail moves behind a disclosure, and no sentence repeats what a chip or a
//! number already says.

use leptos::prelude::*;

use drug_ya_core::model::{LabFlag, LabValue, PregnancyStatus, ReviewResult, Sex};
use drug_ya_core::renal::ckd_stage_thai;
use drug_ya_core::thai;

use crate::actions;
use crate::components::findings_panel::FindingsPanel;
use crate::components::icons::Icon;
use crate::components::orders_table::OrdersPanel;
use crate::state::{AppState, SettingsTab};

/// Main workspace.
#[component]
pub fn ReviewCanvas(state: AppState) -> impl IntoView {
    view! {
        <main class="workspace">
            {move || {
                if let Some(review) = state.review.get() {
                    return view! { <ReviewBody state=state review=review/> }.into_any();
                }
                if state.review_loading.get() {
                    return view! { <ReviewSkeleton/> }.into_any();
                }
                if let Some(error) = state.review_error.get() {
                    return view! {
                        <div class="review-error">
                            <div class="review-error__icon"><Icon name="alert" class="icon"/></div>
                            <h2>"โหลดรายการยาไม่สำเร็จ"</h2>
                            <p>{error}</p>
                            <button class="button button--primary" type="button" on:click=move |_| actions::reload_review(state)>
                                <Icon name="refresh" class="icon"/>
                                "ลองใหม่"
                            </button>
                        </div>
                    }.into_any();
                }
                view! { <EmptyState state=state/> }.into_any()
            }}
        </main>
    }
}

#[component]
fn ReviewBody(state: AppState, review: ReviewResult) -> impl IntoView {
    view! {
        <div class="review-body">
            <PatientHeader state=state review=review.clone()/>
            {move || state.review_error.get().map(|error| view! {
                <div class="inline-error" role="alert">
                    <Icon name="alert" class="icon"/>
                    <span>{error}</span>
                </div>
            })}
            {(!review.warnings.is_empty()).then(|| {
                let warnings = review.warnings.clone();
                view! { <WarningLine warnings=warnings/> }
            })}
            <ClinicalSummary review=review.clone()/>
            <ContextBar state=state review=review.clone()/>
            <div class="review-layout">
                <OrdersPanel state=state/>
                <FindingsPanel state=state/>
            </div>
            <div class="review-disclaimer">
                <Icon name="shield" class="icon"/>
                <span>"OPD วันนี้เท่านั้น"</span>
                <span class="dot-sep">"·"</span>
                <span>"ผลตรวจเป็นข้อท้วงติง ไม่ใช่คำสั่งห้ามจ่าย"</span>
            </div>
        </div>
    }
}

#[component]
fn PatientHeader(state: AppState, review: ReviewResult) -> impl IntoView {
    let patient = review.patient.clone();
    let initials = initials(&patient.full_name_th);
    let unsigned = !review.rule_pack.signed_off;
    view! {
        <section class="patient-header">
            <div class="patient-header__avatar">{initials}</div>
            <div class="patient-header__main">
                <div class="patient-header__name-row">
                    <h1 class="patient-header__name">{patient.full_name_th.clone()}</h1>
                    <span class="patient-header__age">{thai::format_age(patient.age_years)}</span>
                    <span class="patient-header__sex">{patient.sex.label_th()}</span>
                </div>
                <div class="patient-header__meta mono">
                    <span>{format!("HN {}", patient.hn)}</span>
                    <span class="dot-sep">"·"</span>
                    <span>{format!("CID {}", if patient.cid.trim().is_empty() { "—" } else { &patient.cid })}</span>
                    {patient.birth_date.map(|birth| view! {
                        <>
                            <span class="dot-sep">"·"</span>
                            <span>{thai::format_thai_date_short(birth)}</span>
                        </>
                    })}
                </div>
                <div class="patient-header__visits">
                    {if review.visits.is_empty() {
                        view! { <span class="visit-chip visit-chip--empty">"ไม่พบ visit วันนี้"</span> }.into_any()
                    } else {
                        review.visits.iter().map(|visit| view! {
                            <span class="visit-chip">
                                {visit.queue.clone().map(|queue| view! {
                                    <span class="visit-chip__queue">{format!("คิว {queue}")}</span>
                                })}
                                <span>{if visit.department_name.trim().is_empty() { visit.department_code.clone() } else { visit.department_name.clone() }}</span>
                                <span class="visit-chip__time">{visit.time.clone().unwrap_or_else(|| "—".into())}</span>
                            </span>
                        }).collect_view().into_any()
                    }}
                </div>
            </div>
            <div class="patient-header__side">
                {unsigned.then(|| view! {
                    <button
                        class="rule-link"
                        type="button"
                        title="เปิดการรับรองชุดกฎ"
                        on:click=move |_| {
                            state.settings_tab.set(SettingsTab::Signoff);
                            state.settings_open.set(true);
                        }
                    >
                        <span class="rule-link__dot"></span>
                        "กฎยังไม่รับรอง"
                    </button>
                })}
                <div class="patient-header__actions">
                    <span class="checked-at">{format!("ตรวจ {}", review.checked_at)}</span>
                    <button
                        class="icon-button icon-button--surface"
                        type="button"
                        title="โหลดใหม่"
                        aria-label="โหลดข้อมูลผู้ป่วยใหม่"
                        disabled=move || state.review_loading.get()
                        on:click=move |_| actions::reload_review(state)
                    >
                        <span class:spin=move || state.review_loading.get()>
                            <Icon name="refresh" class="icon"/>
                        </span>
                    </button>
                </div>
            </div>
        </section>
    }
}

#[component]
fn WarningLine(warnings: Vec<String>) -> impl IntoView {
    view! {
        <details class="warning-line">
            <summary>
                <Icon name="alert" class="icon"/>
                <span>"ข้อมูลบางส่วนไม่ครบ"</span>
                <span class="warning-line__count">{warnings.len()}</span>
                <span class="warning-line__more">"ดู"</span>
            </summary>
            <ul>
                {warnings.into_iter().map(|warning| view! { <li>{warning}</li> }).collect_view()}
            </ul>
        </details>
    }
}

#[component]
fn ClinicalSummary(review: ReviewResult) -> impl IntoView {
    let clinical = review.clinical;
    let renal_value = match (clinical.egfr, clinical.crcl) {
        (Some(egfr), Some(crcl)) => format!("{egfr:.0} / {crcl:.0}"),
        (Some(egfr), None) => format!("{egfr:.0} / —"),
        (None, Some(crcl)) => format!("— / {crcl:.0}"),
        (None, None) => "—".to_string(),
    };
    let renal_sub = match (clinical.egfr, clinical.crcl) {
        (Some(egfr), Some(_)) => format!("eGFR / CrCl · {}", ckd_stage_thai(egfr)),
        (Some(egfr), None) => format!("eGFR · {}", ckd_stage_thai(egfr)),
        (None, Some(_)) => "CrCl".to_string(),
        (None, None) => "ไม่มีผล creatinine".to_string(),
    };
    let renal_tone = renal_tone(clinical.egfr, clinical.crcl);
    let creatinine_sub = clinical
        .creatinine
        .as_ref()
        .map(|lab| thai::format_thai_date_short(lab.measured_on))
        .unwrap_or_else(|| "ไม่มีผล".into());
    let potassium_sub = clinical
        .potassium
        .as_ref()
        .map(|lab| thai::format_thai_date_short(lab.measured_on))
        .unwrap_or_else(|| "ไม่มีผล".into());
    let inr_sub = clinical
        .inr
        .as_ref()
        .map(|lab| thai::format_thai_date_short(lab.measured_on))
        .unwrap_or_else(|| "ไม่มีผล".into());
    let weight_value = clinical
        .weight_kg
        .map(thai::format_qty)
        .unwrap_or_else(|| "—".into());
    let weight_sub = clinical
        .bmi
        .map(|bmi| format!("BMI {bmi:.1}"))
        .unwrap_or_else(|| "ไม่มีน้ำหนัก".into());
    view! {
        <section class="clinical-summary" aria-label="สรุปผลทางคลินิก">
            <Stat label="ไต" value=renal_value unit="" sub=renal_sub tone=renal_tone/>
            <LabStat label="Creatinine" lab=clinical.creatinine fallback="ไม่มีผล" sub=creatinine_sub/>
            <LabStat label="Potassium" lab=clinical.potassium fallback="ไม่มีผล" sub=potassium_sub/>
            <LabStat label="INR" lab=clinical.inr fallback="ไม่มีผล" sub=inr_sub/>
            <Stat label="น้ำหนัก" value=weight_value unit="kg" sub=weight_sub tone="neutral"/>
        </section>
    }
}

#[component]
fn Stat(
    label: &'static str,
    value: String,
    unit: &'static str,
    sub: String,
    tone: &'static str,
) -> impl IntoView {
    view! {
        <div class=format!("summary-stat summary-stat--{tone}")>
            <span class="summary-stat__label">{label}</span>
            <span class="summary-stat__value">
                {value}
                {(!unit.is_empty()).then(|| view! { <span class="summary-stat__unit">{unit}</span> })}
            </span>
            <span class="summary-stat__sub">{sub}</span>
        </div>
    }
}

#[component]
fn LabStat(
    label: &'static str,
    lab: Option<LabValue>,
    fallback: &'static str,
    sub: String,
) -> impl IntoView {
    let tone = lab
        .as_ref()
        .map(|value| match value.flag {
            LabFlag::High | LabFlag::Low => "warn",
            LabFlag::Normal => "ok",
            LabFlag::Unknown => "neutral",
        })
        .unwrap_or("neutral");
    let value = lab
        .as_ref()
        .map(|value| trim_number(value.value))
        .unwrap_or_else(|| "—".into());
    let unit = lab
        .as_ref()
        .map(|value| value.unit.clone())
        .unwrap_or_else(|| fallback.into());
    view! {
        <div class=format!("summary-stat summary-stat--{tone}")>
            <span class="summary-stat__label">{label}</span>
            <span class="summary-stat__value">
                {value}
                {(!unit.is_empty()).then(|| view! { <span class="summary-stat__unit">{unit}</span> })}
            </span>
            <span class="summary-stat__sub">{sub}</span>
        </div>
    }
}

#[component]
fn ContextBar(state: AppState, review: ReviewResult) -> impl IntoView {
    let clinical = review.clinical;
    let derived_pregnancy = clinical.pregnancy;
    let pregnancy = Memo::new(move |_| state.pregnancy_override.get().unwrap_or(derived_pregnancy));
    let diagnoses: Vec<_> = clinical.diagnoses.iter().take(4).cloned().collect();
    let extra_diagnoses = clinical.diagnoses.len().saturating_sub(4);
    let allergies: Vec<_> = clinical.allergies.iter().take(3).cloned().collect();
    let extra_allergies = clinical.allergies.len().saturating_sub(3);
    let female = clinical.sex == Sex::Female;
    view! {
        <section class="context-bar">
            <div class="context-group context-group--allergy">
                <span class="context-label">"แพ้ยา"</span>
                {if !clinical.allergy_data_available {
                    view! { <span class="context-chip context-chip--unknown">"อ่านประวัติไม่ได้"</span> }.into_any()
                } else if clinical.allergies.is_empty() {
                    view! { <span class="context-chip context-chip--ok">"ไม่พบประวัติ"</span> }.into_any()
                } else {
                    view! {
                        {allergies.into_iter().map(|allergy| {
                            let danger = allergy.force_no_order;
                            let title = if allergy.no_alert {
                                "HOSxP ตั้งค่า no_alert = Y"
                            } else {
                                "ประวัติแพ้ยา"
                            };
                            view! {
                                <span class=if danger { "context-chip context-chip--danger" } else { "context-chip context-chip--allergy" } title=title>
                                    <strong>{allergy.agent.clone()}</strong>
                                    {(!allergy.symptom.trim().is_empty()).then(|| view! {
                                        <span class="context-chip__muted">{allergy.symptom.clone()}</span>
                                    })}
                                    {danger.then(|| view! { <span class="context-chip__flag">"ห้ามสั่ง"</span> })}
                                </span>
                            }
                        }).collect_view()}
                        {(extra_allergies > 0).then(|| view! {
                            <span class="context-chip context-chip--more">{format!("+{extra_allergies}")}</span>
                        })}
                    }.into_any()
                }}
            </div>

            <div class="context-group context-group--dx">
                <span class="context-label">"โรค"</span>
                {if diagnoses.is_empty() {
                    view! { <span class="context-chip context-chip--muted">"ไม่พบรหัสวินิจฉัย"</span> }.into_any()
                } else {
                    view! {
                        {diagnoses.into_iter().map(|diagnosis| view! {
                            <span class="context-chip">
                                <span class="mono">{diagnosis.icd10}</span>
                                {diagnosis.label.map(|label| view! { <span class="context-chip__muted">{label}</span> })}
                            </span>
                        }).collect_view()}
                        {(extra_diagnoses > 0).then(|| view! {
                            <span class="context-chip context-chip--more">{format!("+{extra_diagnoses}")}</span>
                        })}
                    }.into_any()
                }}
            </div>

            {female.then(|| view! {
                <div class="context-group context-group--pregnancy">
                    <span class="context-label">"ตั้งครรภ์"</span>
                    <select
                        class="context-select"
                        aria-label="สถานะตั้งครรภ์"
                        prop:value=move || pregnancy_code(pregnancy.get())
                        on:change=move |ev| {
                            actions::set_pregnancy(state, pregnancy_from_code(&event_target_value(&ev)));
                        }
                    >
                        <option value="unknown">"ไม่ทราบ"</option>
                        <option value="pregnant">"ตั้งครรภ์"</option>
                        <option value="notPregnant">"ไม่ตั้งครรภ์"</option>
                    </select>
                </div>
            })}

            {clinical.pmh.clone().map(|pmh| view! {
                <details class="context-pmh">
                    <summary>"ประวัติเดิม"</summary>
                    <p>{pmh}</p>
                </details>
            })}
        </section>
    }
}

#[component]
fn EmptyState(state: AppState) -> impl IntoView {
    view! {
        <div class="empty-state empty-state--clean">
            <div class="accent-rule">
                <span></span><span></span><span></span><span></span><span></span>
            </div>
            <h1>"ตรวจคำสั่งยาวันนี้"</h1>
            <p>"ค้นด้วย HN, CID, ชื่อ หรือคิว แล้วตรวจแพ้ยา ปฏิกิริยา ไต และขนาดยาในหน้าจอเดียว"</p>
            <div class="empty-state__examples">
                <span><span class="mono">"0012345"</span>"HN"</span>
                <span><span class="mono">"110170…"</span>"CID"</span>
                <span><span class="mono">"12"</span>"คิว"</span>
                <span><span class="mono">"สมชาย"</span>"ชื่อ"</span>
            </div>
            <button class="link-button" type="button" on:click=move |_| {
                state.settings_tab.set(SettingsTab::Signoff);
                state.settings_open.set(true);
            }>
                "ตรวจสอบชุดกฎ"
                <Icon name="chevron" class="icon"/>
            </button>
        </div>
    }
}

#[component]
fn ReviewSkeleton() -> impl IntoView {
    view! {
        <div class="review-skeleton" aria-hidden="true">
            <div class="review-skeleton__header"></div>
            <div class="review-skeleton__strip"></div>
            <div class="review-skeleton__body">
                <div></div><div></div>
            </div>
        </div>
    }
}

fn renal_tone(egfr: Option<f64>, crcl: Option<f64>) -> &'static str {
    let value = egfr.or(crcl);
    match value {
        Some(value) if value < 30.0 => "critical",
        Some(value) if value < 60.0 => "warn",
        Some(_) => "ok",
        None => "neutral",
    }
}

fn trim_number(value: f64) -> String {
    if (value - value.trunc()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn pregnancy_code(status: PregnancyStatus) -> &'static str {
    match status {
        PregnancyStatus::Pregnant => "pregnant",
        PregnancyStatus::NotPregnant => "notPregnant",
        PregnancyStatus::NotApplicable | PregnancyStatus::Unknown => "unknown",
    }
}

fn pregnancy_from_code(code: &str) -> PregnancyStatus {
    match code {
        "pregnant" => PregnancyStatus::Pregnant,
        "notPregnant" => PregnancyStatus::NotPregnant,
        _ => PregnancyStatus::Unknown,
    }
}

fn initials(name: &str) -> String {
    let cleaned = ["นางสาว", "นาง", "นาย", "น.ส.", "ด.ช.", "ด.ญ."]
        .iter()
        .find_map(|title| name.trim().strip_prefix(title))
        .unwrap_or(name.trim())
        .trim();
    let parts: Vec<&str> = cleaned
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .collect();
    let first = parts
        .first()
        .and_then(|part| part.chars().next())
        .unwrap_or('?');
    let second = parts.get(1).and_then(|part| part.chars().next());
    match second {
        Some(second) => format!("{first}{second}"),
        None => first.to_string(),
    }
}
