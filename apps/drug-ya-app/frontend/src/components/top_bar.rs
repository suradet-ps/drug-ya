//! Signed-in top bar.

use leptos::prelude::*;

use crate::api::ConnectionHealth;
use crate::components::icons::Icon;
use crate::state::{AppState, SettingsTab};

/// Top bar with date, connection status, officer, settings, logout.
#[component]
pub fn TopBar(state: AppState) -> impl IntoView {
    let date_label = crate::clock::today_thai();
    view! {
        <header class="top-bar">
            <div class="top-bar__left">
                <div class="top-bar__title">"ตรวจคำสั่งยา"</div>
                <div class="top-bar__divider"></div>
                <div class="top-bar__date">
                    <Icon name="clock" class="icon"/>
                    {date_label}
                </div>
            </div>
            <div class="top-bar__right">
                <div
                    class=move || {
                        let connected = matches!(state.health.get(), ConnectionHealth::Connected);
                        if connected { "status-pill status-pill--on" } else { "status-pill status-pill--off" }
                    }
                    title=move || match state.health.get() {
                        ConnectionHealth::Connected => "เชื่อมต่อ HOSxP แล้ว",
                        ConnectionHealth::Disconnected => "ขาดการเชื่อมต่อ HOSxP",
                        ConnectionHealth::Unconfigured => "ยังไม่ได้ตั้งค่า HOSxP",
                    }
                    role="status"
                    aria-live="polite"
                    aria-label=move || match state.health.get() {
                        ConnectionHealth::Connected => "HOSxP เชื่อมต่อแล้ว",
                        ConnectionHealth::Disconnected => "HOSxP ขาดการเชื่อมต่อ",
                        ConnectionHealth::Unconfigured => "HOSxP ยังไม่ได้ตั้งค่า",
                    }
                >
                    <span class="status-dot"></span>
                    <span>"HOSxP"</span>
                </div>

                <div class="officer-chip" title="ผู้ใช้ที่เข้าสู่ระบบ">
                    <span class="officer-chip__avatar">
                        <Icon name="user" class="icon"/>
                    </span>
                    <strong class="officer-chip__name">
                        {move || state.session.get().map(|u| u.display_name).unwrap_or_default()}
                    </strong>
                </div>

                <button
                    class="icon-button"
                    type="button"
                    title="ตั้งค่ากฎการตรวจ"
                    aria-label="ตั้งค่ากฎการตรวจ"
                    on:click=move |_| {
                        state.settings_tab.set(SettingsTab::Rules);
                        state.settings_open.set(true);
                    }
                >
                    <Icon name="settings" class="icon"/>
                </button>
                <button
                    class="icon-button"
                    type="button"
                    title="ออกจากระบบ"
                    aria-label="ออกจากระบบ"
                    on:click=move |_| crate::actions::logout(state)
                >
                    <Icon name="logout" class="icon"/>
                </button>
            </div>
        </header>
    }
}
