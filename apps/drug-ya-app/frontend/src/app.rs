//! Drug-Ya frontend (Leptos 0.8 CSR).
//!
//! The window starts hidden and is revealed after the shell mounts. The
//! login gate is the first screen; the review console is only rendered for a
//! signed-in officer.

use std::time::Duration;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use crate::components::login_screen::{LoginScreen, StartupSplash};
use crate::components::review_canvas::ReviewCanvas;
use crate::components::search_panel::SearchPanel;
use crate::components::settings_modal::SettingsModal;
use crate::components::top_bar::TopBar;
use crate::state::AppState;

const HEALTH_POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Mount the app.
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| view! { <App/> });
}

#[component]
fn App() -> impl IntoView {
    let state = AppState::new();

    let root_ref = NodeRef::<leptos::html::Div>::new();
    root_ref.on_load(move |_| {
        spawn_local(async move {
            let _ = crate::api::show_main_window().await;
        });
    });

    spawn_local(async move {
        match crate::api::get_app_status().await {
            Ok(status) => {
                state.configured.set(status.configured);
                state.app_status.set(Some(status));
            }
            Err(_) => state.configured.set(false),
        }
        if let Ok(Some(user)) = crate::api::current_session().await {
            state.finish_login(user);
        }
        state.auth_checked.set(true);
    });

    // Poll the backend's live health so the dot reflects a dead database.
    spawn_local(async move {
        loop {
            match crate::api::connection_health().await {
                Ok(health) => state.health.set(health),
                Err(_) => state.health.set(crate::api::ConnectionHealth::Disconnected),
            }
            let delay = js_sys::Promise::new(&mut |resolve, _reject| {
                let callback: &js_sys::Function = resolve.unchecked_ref();
                let _ = web_sys::window()
                    .expect("invariant: Tauri webview window")
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        callback,
                        HEALTH_POLL_INTERVAL.as_millis() as i32,
                    );
            });
            wasm_bindgen_futures::JsFuture::from(delay)
                .await
                .expect("invariant: setTimeout promise resolves");
        }
    });

    view! {
        <div class="app" node_ref=root_ref>
            {move || {
                if !state.auth_checked.get() {
                    return view! { <StartupSplash/> }.into_any();
                }
                if state.session.get().is_none() {
                    return view! { <LoginScreen state=state/> }.into_any();
                }
                view! {
                    <TopBar state=state/>
                    <div class="app__body">
                        <SearchPanel state=state/>
                        <ReviewCanvas state=state/>
                    </div>
                }.into_any()
            }}
            <SettingsModal state=state/>
        </div>
    }
}
