//! Drug-Ya desktop shell (Tauri 2 backend).

pub mod commands;
pub mod state;

use state::AppState;
use std::time::Duration;
use tauri::{Manager, WebviewWindow};

/// Safety net: reveal the window even if the frontend fails to call
/// `show_main_window`, so a broken UI never leaves an invisible process.
const WINDOW_SHOW_FALLBACK: Duration = Duration::from_secs(5);

/// Reveal the main window after the frontend has mounted.
#[tauri::command]
fn show_main_window(window: WebviewWindow) {
    let _ = window.show();
    let _ = window.set_focus();
}

/// Start the Tauri application.
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "drug_ya_app=info,drug_ya_hosxp=info,drug_ya_config=info".into()
            }),
        )
        .init();

    tauri::Builder::default()
        .manage(AppState::new())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(WINDOW_SHOW_FALLBACK).await;
                    let _ = window.show();
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_main_window,
            commands::get_app_status,
            commands::is_configured,
            commands::connection_health,
            commands::save_connection,
            commands::get_connection,
            commands::test_connection,
            commands::clear_connection,
            commands::login,
            commands::logout,
            commands::current_session,
            commands::get_review_settings,
            commands::save_review_settings,
            commands::sign_off_rule_pack,
            commands::search_patients,
            commands::load_review,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Drug-Ya");
}
