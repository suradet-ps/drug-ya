//! Login gate and startup splash.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::icons::{Icon, IconLogo};
use crate::state::AppState;

/// Shown while the startup probe runs.
#[component]
pub fn StartupSplash() -> impl IntoView {
    view! {
        <div class="splash">
            <div class="splash__mark"><IconLogo class="splash__logo"/></div>
            <div class="splash__title">"Drug-Ya"</div>
            <div class="splash__subtitle">"กำลังเตรียมระบบตรวจคำสั่งยา…"</div>
            <div class="splash__bar"><span></span></div>
        </div>
    }
}

/// The login gate - HOSxP officer credentials, with connection setup.
#[component]
pub fn LoginScreen(state: AppState) -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let show_password = RwSignal::new(false);
    let caps_lock = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let username_ref = NodeRef::<leptos::html::Input>::new();
    let password_ref = NodeRef::<leptos::html::Input>::new();

    username_ref.on_load(move |input| {
        let _ = input.focus();
    });

    let open_settings = move |_| {
        state
            .settings_tab
            .set(crate::state::SettingsTab::Connection);
        state.settings_open.set(true);
    };

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if busy.get_untracked() {
            return;
        }
        let user = username.get_untracked();
        let pass = password.get_untracked();
        if user.trim().is_empty() {
            error.set(Some("กรุณากรอกชื่อผู้ใช้".into()));
            if let Some(input) = username_ref.get() {
                let _ = input.focus();
            }
            return;
        }
        if pass.is_empty() {
            error.set(Some("กรุณากรอกรหัสผ่าน".into()));
            if let Some(input) = password_ref.get() {
                let _ = input.focus();
            }
            return;
        }
        busy.set(true);
        error.set(None);
        caps_lock.set(false);
        spawn_local(async move {
            match crate::api::login(&user, &pass).await {
                Ok(session) => {
                    password.set(String::new());
                    state.finish_login(session);
                    state.health.set(crate::api::ConnectionHealth::Connected);
                }
                Err(api_error) => {
                    error.set(Some(api_error.message));
                }
            }
            busy.set(false);
        });
    };

    let connection_label = move || match state.app_status.get() {
        Some(status) if status.configured => {
            if status.site_name.trim().is_empty() {
                "HOSxP พร้อมใช้งาน".to_string()
            } else {
                status.site_name
            }
        }
        _ => "ยังไม่ได้เชื่อมต่อ HOSxP".to_string(),
    };

    view! {
        <div class="login">
            <aside class="login__brand">
                <div class="login__brand-top">
                    <IconLogo class="login__mark"/>
                    <div class="login__brand-copy">
                        <div class="login__brand-name">"Drug-Ya"</div>
                        <div class="login__brand-sub">"ระบบทบทวนคำสั่งยา"</div>
                    </div>
                </div>

                <div class="login__stage">
                    <div class="accent-rule" aria-hidden="true">
                        <span></span><span></span><span></span><span></span><span></span>
                    </div>
                    <h1 class="login__headline">
                        "ตรวจคำสั่งยา"
                        <br/>
                        <span>"ก่อนถึงมือผู้ป่วย"</span>
                    </h1>
                    <LoginPreview/>
                </div>

                <div class="login__footnote">
                    <Icon name="lock" class="icon"/>
                    <span>"อ่าน HOSxP แบบอ่านอย่างเดียว · ไม่เก็บข้อมูลผู้ป่วย"</span>
                </div>
            </aside>

            <main class="login__main">
                <div class="login__toolbar">
                    {move || {
                        state.configured.get().then(|| {
                            let connected = matches!(
                                state.health.get(),
                                crate::api::ConnectionHealth::Connected
                            );
                            view! {
                                <div
                                    class="login__connection"
                                    role="status"
                                    title=if connected {
                                        "เชื่อมต่อ HOSxP แล้ว"
                                    } else {
                                        "ขาดการเชื่อมต่อ HOSxP"
                                    }
                                >
                                    <span
                                        class="status-dot"
                                        class:status-dot--on=connected
                                        class:status-dot--off=!connected
                                    ></span>
                                    <span class="login__connection-text">{connection_label}</span>
                                </div>
                            }
                        })
                    }}
                    <button class="button button--quiet login__settings" type="button" on:click=open_settings>
                        <Icon name="settings" class="icon"/>
                        "ตั้งค่า"
                    </button>
                </div>

                <section class="login__card" aria-labelledby="login-title">
                    <header class="login__card-head">
                        <h2 id="login-title">"เข้าสู่ระบบ"</h2>
                        <p>"ใช้บัญชี officer ของ HOSxP"</p>
                    </header>

                    {move || {
                        (!state.configured.get()).then(|| view! {
                            <div class="login__notice" role="status">
                                <Icon name="alert" class="icon"/>
                                <span>"ยังไม่ได้เชื่อมต่อ HOSxP"</span>
                                <button type="button" on:click=open_settings>"ตั้งค่า"</button>
                            </div>
                        })
                    }}

                    <form class="login__form" on:submit=submit aria-busy=move || busy.get().to_string()>
                        <div class="login__field">
                            <label class="login__label" for="login-username">"ชื่อผู้ใช้"</label>
                            <div class="login__control">
                                <Icon name="user" class="icon"/>
                                <input
                                    id="login-username"
                                    class="login__input"
                                    type="text"
                                    autocomplete="username"
                                    autocapitalize="none"
                                    spellcheck="false"
                                    placeholder="officer login"
                                    node_ref=username_ref
                                    disabled=move || busy.get()
                                    aria-required="true"
                                    aria-describedby="login-error"
                                    prop:value=move || username.get()
                                    on:input=move |ev| {
                                        username.set(event_target_value(&ev));
                                        error.set(None);
                                    }
                                />
                            </div>
                        </div>

                        <div class="login__field">
                            <label class="login__label" for="login-password">"รหัสผ่าน"</label>
                            <div class="login__control">
                                <Icon name="lock" class="icon"/>
                                <input
                                    id="login-password"
                                    class="login__input login__input--password"
                                    prop:type=move || if show_password.get() { "text" } else { "password" }
                                    autocomplete="current-password"
                                    placeholder="••••••••"
                                    node_ref=password_ref
                                    disabled=move || busy.get()
                                    aria-required="true"
                                    aria-describedby="login-error"
                                    prop:value=move || password.get()
                                    on:input=move |ev| {
                                        password.set(event_target_value(&ev));
                                        error.set(None);
                                    }
                                    on:keydown=move |ev| caps_lock.set(ev.get_modifier_state("CapsLock"))
                                    on:keyup=move |ev| caps_lock.set(ev.get_modifier_state("CapsLock"))
                                    on:blur=move |_| caps_lock.set(false)
                                />
                                <button
                                    class="login__reveal"
                                    type="button"
                                    disabled=move || busy.get()
                                    aria-label=move || if show_password.get() { "ซ่อนรหัสผ่าน" } else { "แสดงรหัสผ่าน" }
                                    aria-pressed=move || show_password.get().to_string()
                                    on:click=move |_| show_password.update(|shown| *shown = !*shown)
                                >
                                    {move || if show_password.get() {
                                        view! { <Icon name="eye-off" class="icon"/> }.into_any()
                                    } else {
                                        view! { <Icon name="eye" class="icon"/> }.into_any()
                                    }}
                                </button>
                            </div>
                            {move || caps_lock.get().then(|| view! {
                                <span class="login__caps" role="status">
                                    <Icon name="alert" class="icon"/>
                                    "Caps Lock เปิดอยู่"
                                </span>
                            })}
                        </div>

                        {move || error.get().map(|message| view! {
                            <div class="login__error" id="login-error" role="alert">
                                <Icon name="alert" class="icon"/>
                                <span>{message}</span>
                            </div>
                        })}

                        <button
                            class="button button--primary login__submit"
                            type="submit"
                            disabled=move || busy.get() || !state.configured.get()
                        >
                            {move || busy.get().then(|| view! { <span class="login__spinner"></span> })}
                            <span>
                                {move || if busy.get() { "กำลังเข้าสู่ระบบ…" } else { "เข้าสู่ระบบ" }}
                            </span>
                            {move || (!busy.get()).then(|| view! { <Icon name="chevron" class="icon"/> })}
                        </button>
                    </form>
                </section>
            </main>
        </div>
    }
}

/// A quiet product preview for the brand panel. Decorative only - it contains
/// no patient text and is hidden from assistive technology.
#[component]
fn LoginPreview() -> impl IntoView {
    view! {
        <div class="login-preview" aria-hidden="true">
            <div class="login-preview__card">
                <div class="login-preview__head">
                    <span class="login-preview__avatar"></span>
                    <span class="login-preview__identity">
                        <i class="login-preview__bar login-preview__bar--name"></i>
                        <i class="login-preview__bar login-preview__bar--meta"></i>
                    </span>
                    <span class="login-preview__state"></span>
                </div>

                <div class="login-preview__chips">
                    <span class="login-preview__chip login-preview__chip--danger"></span>
                    <span class="login-preview__chip"></span>
                    <span class="login-preview__chip login-preview__chip--ok"></span>
                </div>

                <div class="login-preview__rows">
                    <div class="login-preview__row">
                        <span class="login-preview__tile login-preview__tile--pink"></span>
                        <span class="login-preview__lines">
                            <i class="login-preview__bar"></i>
                            <i class="login-preview__bar login-preview__bar--short"></i>
                        </span>
                        <span class="login-preview__flag login-preview__flag--critical"></span>
                    </div>
                    <div class="login-preview__row">
                        <span class="login-preview__tile login-preview__tile--purple"></span>
                        <span class="login-preview__lines">
                            <i class="login-preview__bar"></i>
                            <i class="login-preview__bar login-preview__bar--short"></i>
                        </span>
                        <span class="login-preview__flag login-preview__flag--high"></span>
                    </div>
                    <div class="login-preview__row">
                        <span class="login-preview__tile login-preview__tile--blue"></span>
                        <span class="login-preview__lines">
                            <i class="login-preview__bar"></i>
                            <i class="login-preview__bar login-preview__bar--short"></i>
                        </span>
                        <span class="login-preview__flag login-preview__flag--ok"></span>
                    </div>
                </div>

                <div class="login-preview__foot">
                    <span class="login-preview__progress"><i></i></span>
                    <span class="login-preview__action"></span>
                </div>
            </div>
            <span class="login-preview__check"><Icon name="check" class="icon"/></span>
        </div>
    }
}
