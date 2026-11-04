//! Today's medication orders, grouped by visit.

use leptos::prelude::*;

use drug_ya_core::model::{OrderLine, ReviewResult, TodayVisit};
use drug_ya_core::rules::Severity;
use drug_ya_core::thai;

use crate::components::icons::Icon;
use crate::state::AppState;

/// Orders panel.
#[component]
pub fn OrdersPanel(state: AppState) -> impl IntoView {
    view! {
        <section class="panel orders-panel">
            <div class="panel__head">
                <h2 class="panel__title">"รายการยาวันนี้"</h2>
                {move || state.review.get().map(|review| view! {
                    <span class="panel__count">{format!("{} รายการ", review.orders.len())}</span>
                })}
            </div>

            {move || {
                let Some(review) = state.review.get() else {
                    return view! { <OrdersSkeleton/> }.into_any();
                };
                if review.orders.is_empty() {
                    return view! {
                        <div class="orders-empty">
                            <Icon name="pill" class="orders-empty__icon"/>
                            <div class="orders-empty__title">"ยังไม่มีรายการยา"</div>
                            <p>"วันนี้ยังไม่พบคำสั่งยา OPD"</p>
                        </div>
                    }.into_any();
                }
                let visits: Vec<TodayVisit> = review
                    .visits
                    .iter()
                    .filter(|visit| review.orders.iter().any(|order| order.vn == visit.vn))
                    .cloned()
                    .collect();
                view! {
                    <div class="order-groups">
                        <For
                            each=move || visits.clone()
                            key=|visit| visit.vn.clone()
                            children={
                                let review = review.clone();
                                move |visit| {
                                    let orders: Vec<OrderLine> = review
                                        .orders
                                        .iter()
                                        .filter(|order| order.vn == visit.vn)
                                        .cloned()
                                        .collect();
                                    view! {
                                        <VisitGroup visit=visit orders=orders review=review.clone()/>
                                    }
                                }
                            }
                        />
                    </div>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn VisitGroup(visit: TodayVisit, orders: Vec<OrderLine>, review: ReviewResult) -> impl IntoView {
    let count = orders.len();
    view! {
        <div class="visit-group">
            <div class="visit-head">
                <div class="visit-head__left">
                    {visit.queue.clone().map(|queue| view! {
                        <span class="queue-badge">{queue}</span>
                    })}
                    <strong>
                        {if visit.department_name.trim().is_empty() {
                            visit.department_code.clone()
                        } else {
                            visit.department_name.clone()
                        }}
                    </strong>
                    <span>{visit.time.clone().unwrap_or_else(|| "—".into())}</span>
                    <span>{if visit.doctor_name.trim().is_empty() { "ไม่ระบุแพทย์".to_string() } else { visit.doctor_name.clone() }}</span>
                </div>
                <span class="visit-head__count">{count}</span>
            </div>

            <div class="order-table-wrap">
                <table class="order-table">
                    <caption class="sr-only">"รายการยาของวันนี้"</caption>
                    <thead>
                        <tr>
                            <th scope="col">"ยา"</th>
                            <th scope="col">"วิธีใช้"</th>
                            <th scope="col" class="num">"จำนวน"</th>
                            <th scope="col" class="num">"วัน"</th>
                            <th scope="col">"ตรวจ"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || orders.clone()
                            key=|order| order.line_id.clone()
                            children={
                                let review = review.clone();
                                move |order| {
                                    let (severity, finding_count) = line_severity(&review, &order.line_id);
                                    view! { <OrderRow order=order severity=severity finding_count=finding_count/> }
                                }
                            }
                        />
                    </tbody>
                </table>
            </div>
        </div>
    }
}

#[component]
fn OrderRow(order: OrderLine, severity: Option<Severity>, finding_count: usize) -> impl IntoView {
    let row_class = severity
        .map(|severity| format!("order-row order-row--{}", severity.code()))
        .unwrap_or_else(|| "order-row".into());
    let sig_summary = order.sig_summary.trim().to_string();
    let sig_raw = order.sig.trim().to_string();
    let has_summary = !sig_summary.is_empty();
    let has_raw = !sig_raw.is_empty();
    view! {
        <tr class=row_class>
            <td>
                <div class="drug-name">
                    {order.drug_name.clone()}
                    {order.high_alert.then(|| view! {
                        <span class="mini-badge mini-badge--alert">"high-alert"</span>
                    })}
                    {(!order.known).then(|| view! {
                        <span class="mini-badge mini-badge--unknown">"ไม่รู้จัก"</span>
                    })}
                </div>
                <div class="drug-meta">
                    {(!order.strength.trim().is_empty()).then(|| view! { <span class="mono">{order.strength.clone()}</span> })}
                    {order.generic_name.clone().map(|generic| view! { <span>{generic}</span> })}
                </div>
            </td>
            <td class="sig-cell">
                {has_summary.then(|| view! {
                    <div class="sig-summary">{sig_summary.clone()}</div>
                })}
                {if has_raw {
                    view! { <div class="sig-text">{sig_raw.clone()}</div> }.into_any()
                } else if !has_summary {
                    view! {
                        <span class="sig-empty" title="HOSxP ไม่มีข้อความวิธีใช้">"—"</span>
                    }.into_any()
                } else {
                    ().into_any()
                }}
            </td>
            <td class="num mono">{format!("{} {}", thai::format_qty(order.qty), order.units.trim())}</td>
            <td class="num">
                {order.days_supply.map(|days| format!("~{}", thai::format_qty(days))).unwrap_or_else(|| "—".into())}
            </td>
            <td>
                {match severity {
                    Some(severity) => view! {
                        <span
                            class=format!("severity-chip severity-chip--{}", severity.code())
                            title=format!("{finding_count} ข้อท้วงติงในรายการนี้")
                        >
                            {severity.label_th()}
                            {(finding_count > 1).then(|| format!(" {finding_count}"))}
                        </span>
                    }.into_any(),
                    None => view! { <span class="muted">"—"</span> }.into_any(),
                }}
            </td>
        </tr>
    }
}

#[component]
fn OrdersSkeleton() -> impl IntoView {
    view! {
        <div class="orders-skeleton" aria-hidden="true">
            <div class="orders-skeleton__head"></div>
            {(0..4).map(|_| view! { <div class="orders-skeleton__row"></div> }).collect_view()}
        </div>
    }
}

fn line_severity(review: &ReviewResult, line_id: &str) -> (Option<Severity>, usize) {
    let mut severity: Option<Severity> = None;
    let mut count = 0;
    for finding in &review.findings {
        if finding.drug_line_ids.iter().any(|id| id == line_id) {
            count += 1;
            severity = Some(match severity {
                Some(current) if current.rank() <= finding.severity.rank() => current,
                _ => finding.severity,
            });
        }
    }
    (severity, count)
}
