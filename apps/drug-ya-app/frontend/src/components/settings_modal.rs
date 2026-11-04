//! Settings modal: connection, rule pack, and pharmacist sign-off.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use drug_ya_core::rules::{
    Category, RenalMetric, ReviewSettings, RuleConfigView, RuleOverride, Severity,
};

use crate::api::{self, ConnectionInput, ReviewSettingsResponse, SignoffInput};
use crate::components::icons::Icon;
use crate::state::{AppState, SettingsTab};

/// Settings modal.
#[component]
pub fn SettingsModal(state: AppState) -> impl IntoView {
    // Load settings whenever the modal opens for a signed-in officer.
    Effect::new(move |_| {
        if state.settings_open.get()
            && state.session.get().is_some()
            && state.settings_data.get().is_none()
            && !state.settings_loading.get_untracked()
        {
            spawn_local(async move {
                crate::actions::load_settings(state).await;
            });
        }
    });

    let edit = RwSignal::new(None::<ReviewSettings>);
    Effect::new(move |_| {
        if let Some(data) = state.settings_data.get() {
            edit.set(Some(data.settings.clone()));
        }
    });

    let panel_ref = NodeRef::<leptos::html::Div>::new();
    let close_ref = NodeRef::<leptos::html::Button>::new();

    // Focus the dialog when it opens (and re-focus if the node remounts).
    Effect::new(move |_| {
        if state.settings_open.get()
            && let Some(button) = close_ref.get()
        {
            let _ = button.focus();
        }
    });

    let on_modal_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Escape" {
            ev.prevent_default();
            state.settings_open.set(false);
            return;
        }
        if ev.key() != "Tab" {
            return;
        }
        let Some(panel) = panel_ref.get() else {
            return;
        };
        let Ok(nodes) = panel.query_selector_all(
            "button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])",
        ) else {
            return;
        };
        let focusable: Vec<web_sys::HtmlElement> = (0..nodes.length())
            .filter_map(|index| nodes.get(index))
            .filter_map(|node| node.dyn_into::<web_sys::HtmlElement>().ok())
            .collect();
        if focusable.is_empty() {
            return;
        }
        let active = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.active_element());
        let current = active.as_ref().and_then(|element| {
            focusable.iter().position(|candidate| {
                element.is_same_node(Some(candidate.unchecked_ref::<web_sys::Node>()))
            })
        });
        let next = match (ev.shift_key(), current) {
            (true, Some(0)) => focusable.len() - 1,
            (true, Some(index)) => index - 1,
            (true, None) => focusable.len() - 1,
            (false, Some(index)) => (index + 1) % focusable.len(),
            (false, None) => 0,
        };
        ev.prevent_default();
        let _ = focusable[next].focus();
    };

    view! {
        {move || {
            if !state.settings_open.get() {
                return ().into_any();
            }
            let signed_in = state.session.get().is_some();
            view! {
                <div class="modal-backdrop" on:keydown=on_modal_keydown>
                    <button class="modal-backdrop__click" type="button" aria-label="ปิด" on:click=move |_| state.settings_open.set(false)></button>
                    <div
                        class="modal-panel"
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby="settings-title"
                        node_ref=panel_ref
                    >
                        <header class="modal-panel__head">
                            <div class="modal-panel__title">
                                <span class="modal-panel__mark"><Icon name="settings" class="icon"/></span>
                                <div>
                                    <h2 id="settings-title">"ตั้งค่า"</h2>
                                    <p>"การเชื่อมต่อ กฎการตรวจ และการรับรองชุดกฎ"</p>
                                </div>
                            </div>
                            <button
                                class="icon-button"
                                type="button"
                                title="ปิด"
                                aria-label="ปิดการตั้งค่า"
                                node_ref=close_ref
                                on:click=move |_| state.settings_open.set(false)
                            >
                                <Icon name="close" class="icon"/>
                            </button>
                        </header>

                        <nav class="modal-tabs" role="tablist" aria-label="หมวดการตั้งค่า">
                            <TabButton state=state tab=SettingsTab::Connection label="การเชื่อมต่อ" icon="database" enabled=true/>
                            {signed_in.then(|| view! {
                                <>
                                    <TabButton state=state tab=SettingsTab::Rules label="กฎการตรวจ" icon="settings" enabled=true/>
                                    <TabButton state=state tab=SettingsTab::Signoff label="การรับรอง" icon="shield" enabled=true/>
                                </>
                            })}
                        </nav>

                        <div class="modal-body" role="tabpanel">
                            {move || state.settings_error.get().map(|error| view! {
                                <div class="form-error form-error--soft" role="alert">
                                    <Icon name="alert" class="icon"/>
                                    <span>{error}</span>
                                </div>
                            })}
                            {move || match state.settings_tab.get() {
                                SettingsTab::Connection => view! { <ConnectionTab state=state/> }.into_any(),
                                SettingsTab::Rules if signed_in => {
                                    match (state.settings_data.get(), edit.get()) {
                                        (Some(data), Some(_)) => view! {
                                            <RulesTab state=state data=data edit=edit/>
                                        }.into_any(),
                                        _ => view! { <SettingsLoading/> }.into_any(),
                                    }
                                }
                                SettingsTab::Signoff if signed_in => {
                                    match state.settings_data.get() {
                                        Some(data) => view! { <SignoffTab state=state data=data/> }.into_any(),
                                        None => view! { <SettingsLoading/> }.into_any(),
                                    }
                                }
                                _ => view! { <SettingsLoading/> }.into_any(),
                            }}
                        </div>
                    </div>
                </div>
            }.into_any()
        }}
    }
}

#[component]
fn TabButton(
    state: AppState,
    tab: SettingsTab,
    label: &'static str,
    icon: &'static str,
    enabled: bool,
) -> impl IntoView {
    view! {
        <button
            class="modal-tab"
            class:modal-tab--active=move || state.settings_tab.get() == tab
            type="button"
            role="tab"
            aria-selected=move || (state.settings_tab.get() == tab).to_string()
            disabled=!enabled
            on:click=move |_| state.settings_tab.set(tab)
        >
            <Icon name=icon class="icon"/>
            {label}
        </button>
    }
}

#[component]
fn SettingsLoading() -> impl IntoView {
    view! {
        <div class="settings-loading">
            <div class="spinner"></div>
            <span>"กำลังโหลดการตั้งค่า…"</span>
        </div>
    }
}

#[component]
fn ConnectionTab(state: AppState) -> impl IntoView {
    let host = RwSignal::new(String::new());
    let port = RwSignal::new("3306".to_string());
    let database = RwSignal::new(String::new());
    let user = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let message = RwSignal::new(None::<(bool, String)>);
    let latency = RwSignal::new(None::<u64>);

    Effect::new(move |_| {
        if state.settings_open.get() {
            password.set(String::new());
            spawn_local(async move {
                if let Ok(info) = api::get_connection().await {
                    host.set(info.host);
                    port.set(info.port.to_string());
                    database.set(info.database);
                    user.set(info.user);
                }
            });
        }
    });

    let form_input = move || ConnectionInput {
        host: host.get_untracked().trim().to_string(),
        port: port.get_untracked().trim().parse::<u16>().unwrap_or(3306),
        database: database.get_untracked().trim().to_string(),
        user: user.get_untracked().trim().to_string(),
        password: password.get_untracked(),
    };

    let save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        message.set(None);
        latency.set(None);
        let input = form_input();
        let was_signed_in = state.session.get_untracked().is_some();
        spawn_local(async move {
            match api::save_connection(&input).await {
                Ok(()) => {
                    state.configured.set(true);
                    state.health.set(api::ConnectionHealth::Connected);
                    if let Ok(status) = api::get_app_status().await {
                        state.app_status.set(Some(status));
                    }
                    if was_signed_in {
                        // A new connection is a new identity scope. The backend
                        // already cleared its session; do not leave the UI in a
                        // signed-in-looking state.
                        state.clear_session();
                    } else {
                        message.set(Some((true, "บันทึกและเชื่อมต่อ HOSxP สำเร็จ".into())));
                    }
                }
                Err(error) => message.set(Some((false, error.message))),
            }
            password.set(String::new());
            busy.set(false);
        });
    };

    let test = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        message.set(None);
        latency.set(None);
        let input = form_input();
        spawn_local(async move {
            match api::test_connection(Some(&input)).await {
                Ok(result) => {
                    latency.set(Some(result.latency_ms));
                    message.set(Some((true, "เชื่อมต่อได้".into())));
                }
                Err(error) => message.set(Some((false, error.message))),
            }
            busy.set(false);
        });
    };

    view! {
        <form class="settings-form" on:submit=save>
            <div class="settings-section">
                <div class="settings-section__head">
                    <div>
                        <h3>"การเชื่อมต่อ HOSxP"</h3>
                        <p>"ใช้บัญชีฐานข้อมูลแบบ GRANT SELECT เท่านั้นในระบบจริง"</p>
                    </div>
                    <span class="badge badge--readonly"><Icon name="lock" class="icon"/>"read-only"</span>
                </div>
                <div class="form-grid form-grid--connection">
                    <label class="field">
                        <span class="field__label">"Host"</span>
                        <input type="text" placeholder="192.168.1.10" prop:value=move || host.get() on:input=move |ev| host.set(event_target_value(&ev))/>
                    </label>
                    <label class="field field--narrow">
                        <span class="field__label">"Port"</span>
                        <input type="number" placeholder="3306" prop:value=move || port.get() on:input=move |ev| port.set(event_target_value(&ev))/>
                    </label>
                    <label class="field">
                        <span class="field__label">"Database"</span>
                        <input type="text" placeholder="hos" prop:value=move || database.get() on:input=move |ev| database.set(event_target_value(&ev))/>
                    </label>
                    <label class="field">
                        <span class="field__label">"User"</span>
                        <input type="text" placeholder="drugya_ro" prop:value=move || user.get() on:input=move |ev| user.set(event_target_value(&ev))/>
                    </label>
                    <label class="field field--wide">
                        <span class="field__label">"Password"</span>
                        <input type="password" placeholder="••••••••" prop:value=move || password.get() on:input=move |ev| password.set(event_target_value(&ev))/>
                    </label>
                </div>
                <div class="settings-note">
                    <Icon name="info" class="icon"/>
                    <span>"รหัสผ่านถูกเข้ารหัสด้วย encryptman และเก็บใน OS keychain ของเครื่องนี้เท่านั้น"</span>
                </div>
            </div>

            {move || message.get().map(|(ok, text)| view! {
                <div class=if ok { "form-message form-message--ok" } else { "form-message form-message--error" } role="status">
                    <Icon name=if ok { "check" } else { "alert" } class="icon"/>
                    <span>{text}</span>
                    {latency.get().map(|ms| view! { <span class="form-message__meta">{format!("{ms} ms")}</span> })}
                </div>
            })}

            <div class="settings-actions">
                <button class="button button--quiet" type="button" disabled=move || busy.get() on:click=test>
                    <Icon name="database" class="icon"/>
                    "ทดสอบการเชื่อมต่อ"
                </button>
                <button class="button button--primary" type="submit" disabled=move || busy.get()>
                    <Icon name="check" class="icon"/>
                    {move || if busy.get() { "กำลังเชื่อมต่อ…" } else { "บันทึกการเชื่อมต่อ" }}
                </button>
            </div>
        </form>
    }
}

#[component]
fn RulesTab(
    state: AppState,
    data: ReviewSettingsResponse,
    edit: RwSignal<Option<ReviewSettings>>,
) -> impl IntoView {
    let search = RwSignal::new(String::new());
    let pack = data.pack;
    let rules = data.rules;
    let category_enabled = move |category: Category| {
        edit.get()
            .is_some_and(|settings| settings.enabled_categories.contains(&category))
    };
    let set_field = move |update: &dyn Fn(&mut ReviewSettings)| {
        edit.update(|slot| {
            if let Some(settings) = slot {
                update(settings);
            }
        });
    };

    view! {
        <div class="settings-section">
            <div class="settings-section__head">
                <div>
                    <h3>"ชุดกฎการตรวจ"</h3>
                    <p>{format!("{} · v{} · {} กฎ", pack.name, pack.version, pack.rule_count)}</p>
                </div>
                <span class=if pack.signed_off { "badge badge--ok" } else { "badge badge--warn" }>
                    <Icon name=if pack.signed_off { "check" } else { "alert" } class="icon"/>
                    {if pack.signed_off { "รับรองแล้ว" } else { "ยังไม่รับรอง" }}
                </span>
            </div>

            <div class="settings-grid">
                <label class="field">
                    <span class="field__label">"ชื่อสถานบริการ"</span>
                    <input
                        type="text"
                        placeholder="โรงพยาบาล…"
                        prop:value=move || edit.get().map(|s| s.site_name).unwrap_or_default()
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            set_field(&|settings| settings.site_name = value.clone());
                        }
                    />
                </label>
                <label class="field field--narrow">
                    <span class="field__label">"เกณฑ์ผู้สูงอายุ (ปี)"</span>
                    <input
                        type="number"
                        min="18"
                        max="120"
                        prop:value=move || edit.get().map(|s| s.elderly_age.to_string()).unwrap_or_default()
                        on:input=move |ev| {
                            let value = event_target_value(&ev).parse::<u8>().unwrap_or(65);
                            set_field(&|settings| settings.elderly_age = value.clamp(18, 120));
                        }
                    />
                </label>
                <label class="field">
                    <span class="field__label">"ตัวชี้วัดไตที่ใช้"</span>
                    <select
                        prop:value=move || match edit.get().map(|s| s.renal_metric).unwrap_or(RenalMetric::Auto) {
                            RenalMetric::Auto => "auto",
                            RenalMetric::CockcroftGault => "crcl",
                            RenalMetric::CkdEpi => "egfr",
                        }
                        on:change=move |ev| {
                            let value = match event_target_value(&ev).as_str() {
                                "crcl" => RenalMetric::CockcroftGault,
                                "egfr" => RenalMetric::CkdEpi,
                                _ => RenalMetric::Auto,
                            };
                            set_field(&|settings| settings.renal_metric = value);
                        }
                    >
                        <option value="auto">"อัตโนมัติ (CrCl ถ้ามีน้ำหนัก)"</option>
                        <option value="crcl">"Cockcroft-Gault (CrCl)"</option>
                        <option value="egfr">"CKD-EPI 2021 (eGFR)"</option>
                    </select>
                </label>
                <label class="field">
                    <span class="field__label">"แสดงผลขั้นต่ำ"</span>
                    <select
                        prop:value=move || severity_code(edit.get().map(|s| s.min_severity).unwrap_or(Severity::Info))
                        on:change=move |ev| {
                            let value = severity_from_str(&event_target_value(&ev));
                            set_field(&|settings| settings.min_severity = value);
                        }
                    >
                        <option value="critical">"วิกฤต"</option>
                        <option value="high">"สูง"</option>
                        <option value="moderate">"ปานกลาง"</option>
                        <option value="low">"ต่ำ"</option>
                        <option value="info">"ทั้งหมด"</option>
                    </select>
                </label>
            </div>

            <div class="toggle-row">
                <Toggle
                    label="ตรวจการตั้งครรภ์"
                    checked=move || edit.get().is_some_and(|s| s.enable_pregnancy)
                    on_change=move |value| set_field(&|settings| settings.enable_pregnancy = value)
                />
                <Toggle
                    label="ตรวจเด็ก"
                    checked=move || edit.get().is_some_and(|s| s.enable_pediatric)
                    on_change=move |value| set_field(&|settings| settings.enable_pediatric = value)
                />
                <Toggle
                    label="แสดง high-alert"
                    checked=move || edit.get().is_some_and(|s| s.show_high_alert)
                    on_change=move |value| set_field(&|settings| settings.show_high_alert = value)
                />
            </div>

            <div class="category-row">
                {Category::all().iter().map(|category| {
                    let category = *category;
                    view! {
                        <label class="category-toggle">
                            <input
                                type="checkbox"
                                prop:checked=move || category_enabled(category)
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    set_field(&|settings| {
                                        if checked {
                                            if !settings.enabled_categories.contains(&category) {
                                                settings.enabled_categories.push(category);
                                            }
                                        } else {
                                            settings.enabled_categories.retain(|c| *c != category);
                                        }
                                    });
                                }
                            />
                            <span>{category.label_th()}</span>
                        </label>
                    }
                }).collect_view()}
            </div>

            <div class="rules-toolbar">
                <div class="search-box search-box--settings">
                    <Icon name="search" class="search-box__icon"/>
                    <input
                        type="text"
                        placeholder="ค้นหากฎ…"
                        prop:value=move || search.get()
                        on:input=move |ev| search.set(event_target_value(&ev))
                    />
                </div>
                <button class="button button--quiet" type="button" on:click=move |_| {
                    edit.update(|slot| {
                        if let Some(settings) = slot {
                            settings.overrides.clear();
                            settings.enabled_categories = Category::all().to_vec();
                            settings.min_severity = Severity::Info;
                        }
                    });
                }>
                    "คืนค่าเริ่มต้น"
                </button>
            </div>

            <div class="rules-list">
                {move || {
                    let query = search.get().trim().to_lowercase();
                    let rules = rules.clone();
                    Category::all().iter().map(|category| {
                        let category = *category;
                        let filtered: Vec<RuleConfigView> = rules
                            .iter()
                            .filter(|rule| rule.category == category)
                            .filter(|rule| {
                                query.is_empty()
                                    || rule.title.to_lowercase().contains(&query)
                                    || rule.id.contains(&query)
                                    || rule.source.to_lowercase().contains(&query)
                            })
                            .cloned()
                            .collect();
                        if filtered.is_empty() {
                            return ().into_any();
                        }
                        view! {
                            <div class="rule-group">
                                <div class="rule-group__head">
                                    <span>{category.label_th()}</span>
                                    <span>{format!("{} กฎ", filtered.len())}</span>
                                </div>
                                <For
                                    each=move || filtered.clone()
                                    key=|rule| rule.id.clone()
                                    children=move |rule| view! { <RuleRow rule=rule edit=edit/> }
                                />
                            </div>
                        }.into_any()
                    }).collect_view()
                }}
            </div>

            <div class="settings-actions">
                <span class="settings-actions__hint">
                    "การเปลี่ยนแปลงมีผลกับการตรวจครั้งถัดไป"
                </span>
                <button
                    class="button button--primary"
                    type="button"
                    disabled=move || state.settings_loading.get()
                    on:click=move |_| {
                        if let Some(settings) = edit.get_untracked() {
                            crate::actions::save_settings(state, settings);
                        }
                    }
                >
                    <Icon name="check" class="icon"/>
                    {move || if state.settings_loading.get() { "กำลังบันทึก…" } else { "บันทึกกฎ" }}
                </button>
            </div>
        </div>
    }
}

#[component]
fn RuleRow(rule: RuleConfigView, edit: RwSignal<Option<ReviewSettings>>) -> impl IntoView {
    let rule_id = rule.id.clone();
    let category = rule.category;
    let default_enabled = rule.default_enabled;
    let default_severity = rule.severity;
    let default_threshold = rule.threshold_value;
    let threshold_meta = rule.threshold_meta.clone();

    let id_enabled = rule_id.clone();
    let current_enabled = Memo::new(move |_| {
        edit.get().is_some_and(|settings| {
            settings.enabled_categories.contains(&category)
                && settings
                    .overrides
                    .get(&id_enabled)
                    .map(|override_value| override_value.enabled)
                    .unwrap_or(default_enabled)
        })
    });
    let id_severity = rule_id.clone();
    let current_severity = Memo::new(move |_| {
        edit.get()
            .and_then(|settings| {
                settings
                    .overrides
                    .get(&id_severity)
                    .and_then(|o| o.severity)
            })
            .unwrap_or(default_severity)
    });
    let id_threshold = rule_id.clone();
    let current_threshold = Memo::new(move |_| {
        edit.get()
            .and_then(|settings| {
                settings
                    .overrides
                    .get(&id_threshold)
                    .and_then(|o| o.threshold)
            })
            .or(default_threshold)
    });

    let id_switch = rule_id.clone();
    let id_severity_input = rule_id.clone();
    let id_threshold_input = rule_id.clone();

    view! {
        <div class="rule-row" class:rule-row--off=move || !current_enabled.get()>
            <label class="switch">
                <input
                    type="checkbox"
                    prop:checked=move || current_enabled.get()
                    on:change=move |ev| {
                        let enabled = event_target_checked(&ev);
                        update_override(edit, &id_switch, enabled, |override_value| override_value.enabled = enabled);
                    }
                />
                <span class="switch__track"><span class="switch__thumb"></span></span>
            </label>
            <div class="rule-row__body">
                <div class="rule-row__title">{rule.title.clone()}</div>
                <div class="rule-row__meta">
                    <span class="mono">{rule.id.clone()}</span>
                    <span>{rule.source.clone()}</span>
                </div>
            </div>
            <div class="rule-row__controls">
                <select
                    class="compact-select"
                    title="ความรุนแรง"
                    prop:value=move || severity_code(current_severity.get())
                    on:change=move |ev| {
                        let severity = severity_from_str(&event_target_value(&ev));
                        update_override(edit, &id_severity_input, default_enabled, |override_value| override_value.severity = Some(severity));
                    }
                >
                    <option value="critical">"วิกฤต"</option>
                    <option value="high">"สูง"</option>
                    <option value="moderate">"ปานกลาง"</option>
                    <option value="low">"ต่ำ"</option>
                    <option value="info">"ข้อมูล"</option>
                </select>
                {threshold_meta.map(|meta| view! {
                    <label class="threshold-input">
                        <span>{meta.label.clone()}</span>
                        <input
                            type="number"
                            min=meta.min
                            max=meta.max
                            step=meta.step
                            prop:value=move || current_threshold.get().map(format_number).unwrap_or_default()
                            on:change=move |ev| {
                                let raw = event_target_value(&ev);
                                let value = raw.trim().parse::<f64>().ok();
                                update_override(edit, &id_threshold_input, default_enabled, |override_value| override_value.threshold = value);
                            }
                        />
                        <span>{meta.unit.clone()}</span>
                    </label>
                })}
            </div>
        </div>
    }
}

fn update_override(
    edit: RwSignal<Option<ReviewSettings>>,
    rule_id: &str,
    initial_enabled: bool,
    update: impl FnOnce(&mut RuleOverride),
) {
    edit.update(|slot| {
        if let Some(settings) = slot {
            let entry = settings
                .overrides
                .entry(rule_id.to_string())
                .or_insert_with(|| RuleOverride {
                    enabled: initial_enabled,
                    severity: None,
                    threshold: None,
                });
            update(entry);
        }
    });
}

#[component]
fn Toggle(
    label: &'static str,
    checked: impl Fn() -> bool + Copy + Send + Sync + 'static,
    on_change: impl Fn(bool) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <label class="toggle">
            <input type="checkbox" prop:checked=checked on:change=move |ev| on_change(event_target_checked(&ev))/>
            <span class="toggle__track"><span class="toggle__thumb"></span></span>
            <span>{label}</span>
        </label>
    }
}

#[component]
fn SignoffTab(state: AppState, data: ReviewSettingsResponse) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let license = RwSignal::new(String::new());
    let note = RwSignal::new(String::new());
    let confirmed = RwSignal::new(false);
    let pack = data.pack;
    let signoff = pack.signoff.clone();
    let signed = pack.signed_off;

    let submit = move |_| {
        if !confirmed.get_untracked() || state.settings_loading.get_untracked() {
            return;
        }
        let input = SignoffInput {
            pharmacist_name: name.get_untracked(),
            license_no: license.get_untracked(),
            note: note.get_untracked(),
        };
        crate::actions::signoff(state, input);
        confirmed.set(false);
    };

    view! {
        <div class="settings-section signoff">
            <div class="settings-section__head">
                <div>
                    <h3>"การรับรองชุดกฎ"</h3>
                    <p>"เภสัชกรต้องตรวจทานเงื่อนไข แหล่งอ้างอิง และความเหมาะสมกับโรงพยาบาลก่อนใช้จริง"</p>
                </div>
                <span class=if signed { "badge badge--ok" } else { "badge badge--warn" }>
                    <Icon name=if signed { "check" } else { "alert" } class="icon"/>
                    {if signed { "รับรองแล้ว" } else { "ยังไม่รับรอง" }}
                </span>
            </div>

            <div class="pack-card">
                <div class="pack-card__top">
                    <div>
                        <div class="pack-card__name">{pack.name.clone()}</div>
                        <div class="pack-card__meta">
                            {format!("v{} · มีผล {}", pack.version, drug_ya_core::thai::format_thai_date_short(pack.effective_date))}
                        </div>
                    </div>
                    <span class="chip chip--soft">{format!("{} กฎ", pack.rule_count)}</span>
                </div>
                <p class="pack-card__desc">{pack.description.clone()}</p>
                <div class="pack-card__warn">
                    <Icon name="alert" class="icon"/>
                    <span>
                        "ชุดกฎนี้เป็น starter pack ไม่ใช่ฐานข้อมูลอันตรกิริยาทั้งหมด และไม่แทนการตัดสินใจทางคลินิก"
                        " ต้องทดสอบกับผู้ป่วยจำลองและทบทวนรายการยาก่อนใช้จริง"
                    </span>
                </div>
            </div>

            {signoff.clone().map(|signoff| view! {
                <div class="signoff-current">
                    <div class="signoff-current__icon"><Icon name="shield" class="icon"/></div>
                    <div>
                        <strong>{format!("รับรองโดย {}", signoff.pharmacist_name)}</strong>
                        <span>{format!(
                            "เลขใบประกอบ {} · วันที่ {} · กฎ v{}",
                            signoff.license_no,
                            drug_ya_core::thai::format_thai_date_short(signoff.signed_on),
                            signoff.pack_version
                        )}</span>
                        {(!signoff.note.trim().is_empty()).then(|| view! { <p>{signoff.note}</p> })}
                    </div>
                </div>
            })}

            <div class="signoff-form">
                <div class="signoff-form__title">"ลงนามรับรองชุดกฎ"</div>
                <div class="form-grid">
                    <label class="field">
                        <span class="field__label">"ชื่อเภสัชกร"</span>
                        <input type="text" placeholder="ภก. …" prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev))/>
                    </label>
                    <label class="field">
                        <span class="field__label">"เลขใบประกอบวิชาชีพ"</span>
                        <input type="text" placeholder="…" prop:value=move || license.get() on:input=move |ev| license.set(event_target_value(&ev))/>
                    </label>
                    <label class="field field--wide">
                        <span class="field__label">"หมายเหตุ (ไม่บังคับ)"</span>
                        <textarea rows="3" placeholder="ขอบเขตการตรวจ / ข้อจำกัดที่ตกลงกับโรงพยาบาล" prop:value=move || note.get() on:input=move |ev| note.set(event_target_value(&ev))></textarea>
                    </label>
                </div>
                <label class="confirm-check">
                    <input type="checkbox" prop:checked=move || confirmed.get() on:change=move |ev| confirmed.set(event_target_checked(&ev))/>
                    <span>"ข้าพเจ้าได้ตรวจสอบเงื่อนไข แหล่งอ้างอิง และทดสอบกับผู้ป่วยจำลองแล้ว"</span>
                </label>
                <div class="settings-actions">
                    <span class="settings-actions__hint">
                        "การรับรองจะบันทึกชื่อและวันที่ใน settings.json (ไม่ใช่ข้อมูลผู้ป่วย)"
                    </span>
                    <button class="button button--primary" type="button" disabled=move || !confirmed.get() || state.settings_loading.get() on:click=submit>
                        <Icon name="check" class="icon"/>
                        {move || if state.settings_loading.get() { "กำลังบันทึก…" } else { "ลงนามรับรอง" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

fn severity_code(severity: Severity) -> &'static str {
    severity.code()
}

fn severity_from_str(value: &str) -> Severity {
    match value {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "moderate" => Severity::Moderate,
        "low" => Severity::Low,
        _ => Severity::Info,
    }
}

fn format_number(value: f64) -> String {
    if (value - value.trunc()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}
