//! Patient search rail: HN / CID / name / today's queue.

use std::time::Duration;

use leptos::prelude::*;

use drug_ya_core::query_kind::{SearchKind, parse_search};
use drug_ya_core::thai;

use crate::actions;
use crate::components::icons::Icon;
use crate::state::AppState;

/// Search panel.
#[component]
pub fn SearchPanel(state: AppState) -> impl IntoView {
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let active_index = RwSignal::new(0usize);

    input_ref.on_load(move |input| {
        let _ = input.focus();
    });

    // Keep keyboard selection aligned with fresh results.
    Effect::new(move |_| {
        state.search_results.get();
        active_index.set(0);
    });

    let clear_search = move || {
        state.search_query.set(String::new());
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    };

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        match key.as_str() {
            "Escape" => {
                if !state.search_query.get_untracked().is_empty() {
                    ev.prevent_default();
                    clear_search();
                }
            }
            "ArrowDown" => {
                let len = state.search_results.get_untracked().len();
                if len > 0 {
                    ev.prevent_default();
                    active_index.update(|index| *index = (*index + 1) % len);
                }
            }
            "ArrowUp" => {
                let len = state.search_results.get_untracked().len();
                if len > 0 {
                    ev.prevent_default();
                    active_index.update(|index| {
                        *index = if *index == 0 { len - 1 } else { *index - 1 };
                    });
                }
            }
            "Enter" => {
                if state.search_loading.get_untracked() {
                    return;
                }
                let results = state.search_results.get_untracked();
                if let Some(patient) = results.get(active_index.get_untracked()).cloned() {
                    ev.prevent_default();
                    actions::select_patient(state, patient);
                }
            }
            _ => {}
        }
    };

    // Debounce + generation guard: a stale timeout never fires a request.
    Effect::new(move |_| {
        let query = state.search_query.get();
        let kind = state.search_kind.get();
        let term = query.trim().to_string();
        let token = state.search_epoch.get_untracked().wrapping_add(1);
        state.search_epoch.set(token);
        state.search_error.set(None);
        if term.is_empty() {
            state.search_results.set(Vec::new());
            state.search_error.set(None);
            state.search_loading.set(false);
            return;
        }
        let enough = match kind {
            SearchKind::Queue => true,
            SearchKind::Hn | SearchKind::Cid => term.chars().count() >= 5,
            SearchKind::Name => term.chars().count() >= 2,
            SearchKind::Auto => {
                let parsed = parse_search(&term, SearchKind::Auto);
                match parsed.kind {
                    SearchKind::Queue => true,
                    SearchKind::Hn | SearchKind::Cid => parsed.term.chars().count() >= 5,
                    _ => term.chars().count() >= 2,
                }
            }
        };
        if !enough {
            state.search_results.set(Vec::new());
            state.search_loading.set(false);
            return;
        }
        set_timeout(
            move || {
                if state.search_epoch.get_untracked() == token {
                    actions::start_search(state);
                }
            },
            Duration::from_millis(320),
        );
    });

    view! {
        <aside class="search-rail">
            <div class="brand-lockup brand-lockup--rail">
                <crate::components::icons::IconLogo class="brand-lockup__mark"/>
                <div>
                    <div class="brand-lockup__name">"Drug-Ya"</div>
                    <div class="brand-lockup__tagline">"ตรวจคำสั่งยาวันนี้"</div>
                </div>
            </div>

            <div class="rail-section">
                <div class="rail-section__head">
                    <span class="rail-section__label">"ค้นหาผู้ป่วย"</span>
                    <span class="rail-section__hint">"HN · CID · ชื่อ · คิว"</span>
                </div>
                <div class="search-box">
                    <Icon name="search" class="search-box__icon"/>
                    <input
                        type="text"
                        spellcheck="false"
                        autocomplete="off"
                        placeholder="HN, CID, ชื่อ หรือ Q12"
                        aria-label="ค้นหาผู้ป่วย"
                        aria-controls="patient-results"
                        enterkeyhint="search"
                        node_ref=input_ref
                        prop:value=move || state.search_query.get()
                        on:input=move |ev| state.search_query.set(event_target_value(&ev))
                        on:keydown=on_keydown
                    />
                    {move || (!state.search_query.get().is_empty()).then(|| view! {
                        <button
                            class="search-box__clear"
                            type="button"
                            title="ล้างคำค้นหา"
                            aria-label="ล้างคำค้นหา"
                            on:click=move |_| clear_search()
                        >
                            <Icon name="close" class="icon"/>
                        </button>
                    })}
                </div>

                <div class="search-mode">
                    <select
                        class="search-mode__select"
                        aria-label="โหมดค้นหา"
                        prop:value=move || match state.search_kind.get() {
                            SearchKind::Auto => "auto",
                            SearchKind::Hn => "hn",
                            SearchKind::Cid => "cid",
                            SearchKind::Name => "name",
                            SearchKind::Queue => "queue",
                        }
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            state.search_kind.set(match value.as_str() {
                                "hn" => SearchKind::Hn,
                                "cid" => SearchKind::Cid,
                                "name" => SearchKind::Name,
                                "queue" => SearchKind::Queue,
                                _ => SearchKind::Auto,
                            });
                        }
                    >
                        <option value="auto">"อัตโนมัติ"</option>
                        <option value="hn">"HN"</option>
                        <option value="cid">"CID"</option>
                        <option value="name">"ชื่อ"</option>
                        <option value="queue">"คิว"</option>
                    </select>
                    <div class="search-mode__hint">
                        {move || search_hint(state.search_query.get(), state.search_kind.get())}
                    </div>
                </div>
            </div>

            <div class="rail-section rail-section--results">
                {move || {
                    (!state.search_query.get().trim().is_empty()).then(|| view! {
                        <div class="rail-section__head">
                            <span class="rail-section__label">"ผลการค้นหา"</span>
                            <span class="rail-section__count" aria-live="polite">
                                {move || {
                                    let count = state.search_results.get().len();
                                    if count > 0 { format!("{count} ราย") } else { String::new() }
                                }}
                            </span>
                        </div>
                    })
                }}

                <div class="result-list" id="patient-results" aria-busy=move || state.search_loading.get().to_string()>
                    {move || {
                        if state.search_loading.get() && state.search_results.get().is_empty() {
                            return view! { <ResultSkeleton/> }.into_any();
                        }
                        if let Some(error) = state.search_error.get() {
                            return view! {
                                <div class="rail-error" role="alert">
                                    <Icon name="alert" class="icon"/>
                                    <span>{error}</span>
                                </div>
                            }.into_any();
                        }
                        if state.search_results.get().is_empty() {
                            if state.search_query.get().trim().is_empty() {
                                return view! { <RailIdle/> }.into_any();
                            }
                            if state.search_loading.get() {
                                return view! { <ResultSkeleton/> }.into_any();
                            }
                            return view! {
                                <div class="rail-empty">
                                    <Icon name="search" class="rail-empty__icon"/>
                                    <div>"ไม่พบผู้ป่วย"</div>
                                    <span>"ลองตรวจรูปแบบ HN/CID หรือเลือกโหมดค้นหา"</span>
                                </div>
                            }.into_any();
                        }
                        view! {
                            <div class="result-items">
                                {state.search_results.get().into_iter().enumerate().map(|(index, patient)| {
                                    let selected = {
                                        let hn = patient.hn.clone();
                                        Memo::new(move |_| {
                                            state.selected_patient.get().is_some_and(|p| p.hn == hn)
                                        })
                                    };
                                    let is_active = move || selected.get() || active_index.get() == index;
                                    let aria_label = format!(
                                        "เลือก {} · HN {}",
                                        patient.full_name_th,
                                        patient.hn
                                    );
                                    let patient_for_click = patient.clone();
                                    view! {
                                        <button
                                            class="result-item"
                                            class:result-item--active=is_active
                                            type="button"
                                            aria-current=move || if is_active() { "true" } else { "false" }
                                            aria-label=aria_label
                                            on:mouseenter=move |_| active_index.set(index)
                                            on:click=move |_| actions::select_patient(state, patient_for_click.clone())
                                        >
                                            <span class="result-item__avatar">
                                                {initials(&patient.full_name_th)}
                                            </span>
                                            <span class="result-item__body">
                                                <span class="result-item__name">{patient.full_name_th.clone()}</span>
                                                <span class="result-item__meta">
                                                    <span class="mono">{format!("HN {}", patient.hn)}</span>
                                                    <span class="dot-sep">"·"</span>
                                                    <span>{thai::format_age(patient.age_years)}</span>
                                                    <span class="dot-sep">"·"</span>
                                                    <span>{patient.sex.label_th()}</span>
                                                </span>
                                                <span class="result-item__cid mono">
                                                    {format!("CID {}", mask_cid(&patient.cid))}
                                                </span>
                                            </span>
                                            <Icon name="chevron" class="result-item__chevron"/>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </div>
            </div>

            <div class="rail-footer">
                <div class="rail-footer__badge">
                    <Icon name="shield" class="icon"/>
                    <span>"อ่าน HOSxP แบบ read-only"</span>
                </div>
                <div class="rail-footer__note">
                    "ข้อมูลผู้ป่วยไม่ถูกบันทึกลงเครื่อง · ปิดแอปเมื่อเสร็จงาน"
                </div>
            </div>
        </aside>
    }
}

#[component]
fn RailIdle() -> impl IntoView {
    view! {
        <div class="rail-idle">
            <span class="rail-idle__icon"><Icon name="search" class="icon"/></span>
            <span>"ผลการค้นหาจะแสดงที่นี่"</span>
        </div>
    }
}

#[component]
fn ResultSkeleton() -> impl IntoView {
    view! {
        <div class="result-skeleton" aria-hidden="true">
            {(0..3).map(|_| view! {
                <div class="result-skeleton__row">
                    <span class="result-skeleton__avatar"></span>
                    <span class="result-skeleton__lines">
                        <span></span><span></span>
                    </span>
                </div>
            }).collect_view()}
        </div>
    }
}

fn search_hint(query: String, kind: SearchKind) -> String {
    if query.trim().is_empty() {
        return "ระบบจะตรวจจับประเภทให้อัตโนมัติ".into();
    }
    let parsed = parse_search(&query, kind);
    match parsed.kind {
        SearchKind::Hn => format!("จะค้นด้วย HN: {}", parsed.term),
        SearchKind::Cid => "จะค้นด้วยเลขบัตรประชาชน 13 หลัก".into(),
        SearchKind::Name => format!("จะค้นด้วยชื่อ: {}", parsed.term),
        SearchKind::Queue => format!("จะค้นด้วยคิววันนี้: {}", parsed.term),
        SearchKind::Auto => "ระบบจะตรวจจับประเภทให้อัตโนมัติ".into(),
    }
}

fn mask_cid(cid: &str) -> String {
    let trimmed = cid.trim();
    if trimmed.is_empty() {
        return "ไม่มีข้อมูล".into();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 4 {
        return "••••".into();
    }
    let last: String = chars[chars.len().saturating_sub(4)..].iter().collect();
    format!("•••••••••{last}")
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
