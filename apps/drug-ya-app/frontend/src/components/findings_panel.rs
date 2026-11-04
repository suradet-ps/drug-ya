//! Findings, filter tabs, compact coverage, and pharmacist confirmation.

use leptos::prelude::*;

use drug_ya_core::model::{CoverageReport, Finding, ReviewResult, RiskLevel};
use drug_ya_core::rules::{Category, CheckStatus, Severity};

use crate::components::icons::Icon;
use crate::state::{AppState, FindingFilter};

/// Findings panel.
#[component]
pub fn FindingsPanel(state: AppState) -> impl IntoView {
    view! {
        <section class="panel findings-panel">
            {move || match state.review.get() {
                Some(review) => view! { <FindingsContent state=state review=review/> }.into_any(),
                None => view! { <FindingsSkeleton/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn FindingsContent(state: AppState, review: ReviewResult) -> impl IntoView {
    let summary = review.summary.clone();
    let action = if summary.critical > 0 {
        "ติดต่อแพทย์"
    } else if summary.high > 0 {
        "ทบทวนก่อนจ่าย"
    } else if summary.moderate > 0 {
        "เฝ้าระวัง"
    } else if summary.info > 0 {
        "ตรวจซ้ำ high-alert"
    } else {
        "ไม่พบข้อท้วงติง"
    };
    let has_findings = !review.findings.is_empty();
    let review_for_filter = review.clone();
    let all_acknowledged = {
        let review_for_ack = review.clone();
        Memo::new(move |_| {
            let required: Vec<String> = review_for_ack
                .findings
                .iter()
                .filter(|finding| finding.requires_ack)
                .map(|finding| finding.id.clone())
                .collect();
            let acknowledged = state.acknowledged.get();
            required.iter().all(|id| acknowledged.contains(id))
        })
    };
    let required_count = review.findings.iter().filter(|f| f.requires_ack).count();
    let acknowledged_count = {
        let required: Vec<String> = review
            .findings
            .iter()
            .filter(|f| f.requires_ack)
            .map(|f| f.id.clone())
            .collect();
        Memo::new(move |_| {
            state
                .acknowledged
                .get()
                .iter()
                .filter(|id| required.iter().any(|required| required == *id))
                .count()
        })
    };

    view! {
        <div class="findings-head">
            <h2 class="panel__title">"ผลตรวจ"</h2>
            <span class=format!("findings-action findings-action--{}", risk_class(summary.risk_level))>
                {action}
            </span>
        </div>

        {has_findings.then(|| view! {
            <div class="findings-summary">
                {(summary.critical > 0).then(|| view! {
                    <span class="summary-count summary-count--critical"><i></i>{format!("วิกฤต {}", summary.critical)}</span>
                })}
                {(summary.high > 0).then(|| view! {
                    <span class="summary-count summary-count--high"><i></i>{format!("สูง {}", summary.high)}</span>
                })}
                {(summary.moderate > 0).then(|| view! {
                    <span class="summary-count summary-count--moderate"><i></i>{format!("เฝ้าระวัง {}", summary.moderate)}</span>
                })}
                {(summary.low > 0).then(|| view! {
                    <span class="summary-count summary-count--low"><i></i>{format!("ต่ำ {}", summary.low)}</span>
                })}
                {(summary.info > 0).then(|| view! {
                    <span class="summary-count summary-count--info"><i></i>{format!("ข้อมูล {}", summary.info)}</span>
                })}
            </div>
        })}

        <div class="finding-filters" role="tablist" aria-label="ตัวกรองผลการตรวจ">
            <FilterButton state=state filter=FindingFilter::All count=review.findings.len()/>
            <FilterButton state=state filter=FindingFilter::Action count=count_for(&review, FindingFilter::Action)/>
            <FilterButton state=state filter=FindingFilter::Monitor count=count_for(&review, FindingFilter::Monitor)/>
            <FilterButton state=state filter=FindingFilter::Info count=count_for(&review, FindingFilter::Info)/>
        </div>

        {move || {
            let filter = state.finding_filter.get();
            let filtered: Vec<Finding> = review_for_filter
                .findings
                .iter()
                .filter(|finding| filter_matches(filter, finding.severity))
                .cloned()
                .collect();
            if !has_findings {
                return view! {
                    <div class="all-clear">
                        <div class="all-clear__icon"><Icon name="check" class="icon"/></div>
                        <div class="all-clear__title">"ไม่พบข้อท้วงติง"</div>
                        <p>{format!(
                            "ตรวจ {} / {} หมวด · ยาที่รู้จัก {} / {}",
                            review_for_filter.coverage.categories.iter().filter(|c| c.status == CheckStatus::Checked).count(),
                            review_for_filter.coverage.categories.len(),
                            review_for_filter.coverage.known_drug_count,
                            review_for_filter.coverage.total_order_count
                        )}</p>
                    </div>
                }.into_any();
            }
            if filtered.is_empty() {
                return view! {
                    <div class="filter-empty">
                        <Icon name="filter" class="filter-empty__icon"/>
                        <div>"ไม่มีรายการในตัวกรองนี้"</div>
                        <button class="link-button" type="button" on:click=move |_| state.finding_filter.set(FindingFilter::All)>
                            "แสดงทั้งหมด"
                        </button>
                    </div>
                }.into_any();
            }
            view! {
                <div class="finding-list" id="finding-list" role="tabpanel">
                    <For
                        each=move || filtered.clone()
                        key=|finding| finding.id.clone()
                        children={
                            let acknowledged = state.acknowledged;
                            move |finding| {
                                let finding_id = finding.id.clone();
                                let finding_for_ack = finding.clone();
                                let is_ack = {
                                    let finding_id = finding_id.clone();
                                    Memo::new(move |_| acknowledged.get().contains(&finding_id))
                                };
                                let on_ack = Callback::new(move |_| {
                                    let id = finding_for_ack.id.clone();
                                    acknowledged.update(|set| {
                                        if !set.remove(&id) {
                                            set.insert(id);
                                        }
                                    });
                                    state.review_confirmed.set(None);
                                });
                                view! { <FindingCard finding=finding acknowledged=is_ack on_ack=on_ack/> }
                            }
                        }
                    />
                </div>
            }.into_any()
        }}

        <CoveragePanel coverage=review.coverage.clone()/>

        <div class="review-footer">
            <div class="review-footer__status">
                <span aria-live="polite" class=move || {
                    if acknowledged_count.get() == required_count && required_count > 0 {
                        "review-progress review-progress--done"
                    } else {
                        "review-progress"
                    }
                }>
                    {move || if required_count > 0 {
                        format!("รับทราบ {}/{}", acknowledged_count.get(), required_count)
                    } else {
                        "พร้อมยืนยัน".to_string()
                    }}
                </span>
                <details class="review-note-wrap">
                    <summary>{move || if state.reviewer_note.get().trim().is_empty() { "บันทึก" } else { "บันทึกแล้ว" }}</summary>
                    <textarea
                        rows="2"
                        aria-label="บันทึกสำหรับเภสัชกร"
                        placeholder="สิ่งที่แจ้งแพทย์…"
                        prop:value=move || state.reviewer_note.get()
                        on:input=move |ev| {
                            state.reviewer_note.set(event_target_value(&ev));
                            state.review_confirmed.set(None);
                        }
                    ></textarea>
                </details>
            </div>
            <button
                class="button button--primary"
                type="button"
                disabled=move || !all_acknowledged.get() || state.review_confirmed.get().is_some()
                on:click=move |_| {
                    state.review_confirmed.set(Some(crate::clock::stamp_thai()));
                }
            >
                <Icon name="check" class="icon"/>
                {move || if state.review_confirmed.get().is_some() { "ยืนยันแล้ว" } else { "ยืนยันการทบทวน" }}
            </button>
        </div>
        {move || state.review_confirmed.get().map(|stamp| view! {
            <div class="review-confirmed" role="status">
                <Icon name="check" class="icon"/>
                {format!("ยืนยันแล้ว {stamp}")}
            </div>
        })}
    }
}

#[component]
fn FilterButton(state: AppState, filter: FindingFilter, count: usize) -> impl IntoView {
    view! {
        <button
            class="filter-button"
            class:filter-button--active=move || state.finding_filter.get() == filter
            type="button"
            role="tab"
            aria-controls="finding-list"
            aria-selected=move || (state.finding_filter.get() == filter).to_string()
            on:click=move |_| state.finding_filter.set(filter)
        >
            {filter.label_th()}
            <span class="filter-button__count">{count}</span>
        </button>
    }
}

#[component]
fn FindingCard(finding: Finding, acknowledged: Memo<bool>, on_ack: Callback<()>) -> impl IntoView {
    let severity = finding.severity;
    let category = finding.category;
    let icon = category_icon(category);
    let accent = category_accent(category);
    let card_class = move || {
        format!(
            "finding-card finding-card--{}{}",
            severity.code(),
            if acknowledged.get() {
                " finding-card--ack"
            } else {
                ""
            }
        )
    };
    let source = format!("{} · {}", finding.rule_id, finding.source);
    view! {
        <article class=card_class>
            <div class="finding-card__bar"></div>
            <div class="finding-card__body">
                <div class="finding-card__top">
                    <span class="finding-card__category">
                        <span class=format!("finding-card__icon finding-card__icon--{accent}")><Icon name=icon class="icon"/></span>
                        {category.label_th()}
                    </span>
                    <span class=format!("severity-chip severity-chip--{}", severity.code())>{severity.label_th()}</span>
                </div>

                <h3 class="finding-card__title">{finding.title.clone()}</h3>
                <p class="finding-card__detail">{finding.detail.clone()}</p>

                {(!finding.patient_factors.is_empty()).then(|| view! {
                    <div class="finding-factors">
                        {finding.patient_factors.iter().map(|factor| view! {
                            <span class="factor-chip">{factor.clone()}</span>
                        }).collect_view()}
                    </div>
                })}

                {(!finding.drug_names.is_empty()).then(|| view! {
                    <div class="finding-drugs">
                        {finding.drug_names.iter().map(|drug| view! {
                            <span class="drug-chip">{drug.clone()}</span>
                        }).collect_view()}
                    </div>
                })}

                <div class="finding-recommendation">
                    <span class="finding-recommendation__label">"แนวทาง"</span>
                    <p>{finding.recommendation.clone()}</p>
                </div>

                {finding.data_note.clone().map(|note| view! {
                    <div class="finding-datanote">
                        <Icon name="info" class="icon"/>
                        <span>{note}</span>
                    </div>
                })}

                <details class="finding-evidence">
                    <summary>"ที่มา"</summary>
                    <div class="finding-evidence__body">
                        <p>{finding.evidence.clone()}</p>
                        <span class="mono">{source}</span>
                    </div>
                </details>

                {finding.requires_ack.then(|| view! {
                    <div class="finding-card__actions">
                        <button
                            class="button button--quiet"
                            class:button--ack=move || acknowledged.get()
                            type="button"
                            aria-pressed=move || acknowledged.get().to_string()
                            on:click=move |_| on_ack.run(())
                        >
                            <Icon name="check" class="icon"/>
                            {move || if acknowledged.get() { "รับทราบแล้ว" } else { "รับทราบ" }}
                        </button>
                    </div>
                })}
            </div>
        </article>
    }
}

#[component]
fn CoveragePanel(coverage: CoverageReport) -> impl IntoView {
    let checked = coverage
        .categories
        .iter()
        .filter(|category| category.status == CheckStatus::Checked)
        .count();
    let missing = coverage.missing_data.len();
    view! {
        <details class="coverage">
            <summary class="coverage__summary">
                <Icon name="info" class="icon"/>
                <span>"ความครบถ้วน " {checked} "/" {coverage.categories.len()} " หมวด"</span>
                {(missing > 0).then(|| view! {
                    <span class="coverage__missing">{format!("ข้อมูลขาด {missing}")}</span>
                })}
                <span class="coverage__toggle"><Icon name="chevron" class="icon"/></span>
            </summary>
            <div class="coverage__body">
                <div class="coverage-grid">
                    {coverage.categories.iter().map(|category| {
                        let (class, label) = match category.status {
                            CheckStatus::Checked => ("coverage-row coverage-row--checked", "ตรวจแล้ว"),
                            CheckStatus::Partial => ("coverage-row coverage-row--partial", "บางส่วน"),
                            CheckStatus::Skipped => ("coverage-row coverage-row--skipped", "ข้าม"),
                        };
                        view! {
                            <div class=class>
                                <div class="coverage-row__top">
                                    <span class="coverage-row__name">{category.category.label_th()}</span>
                                    <span class="coverage-row__status">{label}</span>
                                </div>
                                <div class="coverage-row__note">{category.note.clone()}</div>
                            </div>
                        }
                    }).collect_view()}
                </div>

                {(!coverage.missing_data.is_empty()).then(|| view! {
                    <div class="coverage-block">
                        <div class="coverage-block__title">"ข้อมูลที่ขาด"</div>
                        <div class="missing-list">
                            {coverage.missing_data.iter().map(|missing| view! {
                                <div class="missing-item">
                                    <span>{missing.label.clone()}</span>
                                    <span class="missing-item__detail">{missing.detail.clone()}</span>
                                </div>
                            }).collect_view()}
                        </div>
                    </div>
                })}

                {(!coverage.unresolved_drugs.is_empty()).then(|| view! {
                    <div class="coverage-block">
                        <div class="coverage-block__title">"ยาที่ยังไม่รู้จัก"</div>
                        <div class="unresolved-list">
                            {coverage.unresolved_drugs.iter().map(|drug| view! {
                                <span class="factor-chip factor-chip--warn">{drug.clone()}</span>
                            }).collect_view()}
                        </div>
                    </div>
                })}

                {(!coverage.notes.is_empty()).then(|| view! {
                    <ul class="coverage-notes">
                        {coverage.notes.iter().map(|note| view! { <li>{note.clone()}</li> }).collect_view()}
                    </ul>
                })}
            </div>
        </details>
    }
}

fn count_for(review: &ReviewResult, filter: FindingFilter) -> usize {
    review
        .findings
        .iter()
        .filter(|finding| filter_matches(filter, finding.severity))
        .count()
}

fn filter_matches(filter: FindingFilter, severity: Severity) -> bool {
    match filter {
        FindingFilter::All => true,
        FindingFilter::Action => matches!(severity, Severity::Critical | Severity::High),
        FindingFilter::Monitor => matches!(severity, Severity::Moderate | Severity::Low),
        FindingFilter::Info => matches!(severity, Severity::Info),
    }
}

fn risk_class(risk: RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Urgent => "urgent",
        RiskLevel::Review => "review",
        RiskLevel::Monitor => "monitor",
        RiskLevel::Clear => "clear",
    }
}

fn category_icon(category: Category) -> &'static str {
    match category {
        Category::Allergy => "shield",
        Category::Interaction => "interaction",
        Category::DuplicateTherapy => "pill",
        Category::Renal => "kidney",
        Category::Dose => "flask",
        Category::Geriatric => "user",
        Category::Disease => "heart",
        Category::Lab => "flask",
        Category::Pregnancy => "pregnancy",
        Category::Pediatric => "baby",
        Category::HighAlert => "alert",
    }
}

fn category_accent(category: Category) -> &'static str {
    match category {
        Category::Allergy => "pink",
        Category::Interaction => "purple",
        Category::DuplicateTherapy => "orange",
        Category::Renal => "blue",
        Category::Dose => "orange",
        Category::Geriatric => "green",
        Category::Disease => "pink",
        Category::Lab => "blue",
        Category::Pregnancy => "purple",
        Category::Pediatric => "green",
        Category::HighAlert => "yellow",
    }
}

#[component]
fn FindingsSkeleton() -> impl IntoView {
    view! {
        <div class="findings-skeleton" aria-hidden="true">
            <div class="findings-skeleton__hero"></div>
            <div class="findings-skeleton__row"></div>
            <div class="findings-skeleton__row"></div>
            <div class="findings-skeleton__row"></div>
        </div>
    }
}
