//! Drug-Ya desktop shell (Tauri 2 backend).

pub mod commands;
pub mod state;

use state::AppState;
use std::time::Duration;
use tauri::{LogicalPosition, LogicalSize, Manager, WebviewWindow};

/// Safety net: reveal the window even if the frontend fails to call
/// `show_main_window`, so a broken UI never leaves an invisible process.
const WINDOW_SHOW_FALLBACK: Duration = Duration::from_secs(5);

const PREFERRED_WIDTH: f64 = 1440.0;
const PREFERRED_HEIGHT: f64 = 900.0;
const MIN_WIDTH: f64 = 960.0;
const MIN_HEIGHT: f64 = 600.0;
// Conservative Windows decoration allowance. The window is hidden while this
// runs, so the final size and position are applied before the first paint.
const FRAME_WIDTH: f64 = 16.0;
const FRAME_HEIGHT: f64 = 40.0;

/// Fit the hidden window inside the monitor work area and center it there.
///
/// The old 1440x900 default was taller than a 1366x768 work area and could
/// open with the bottom edge behind the taskbar. Use at most the preferred
/// size, but always leave room for the title bar and window borders.
fn fit_and_center(window: &WebviewWindow) -> tauri::Result<()> {
    let monitor = match window.current_monitor()? {
        Some(monitor) => Some(monitor),
        None => window.primary_monitor()?,
    };
    let Some(monitor) = monitor else {
        window.center()?;
        return Ok(());
    };

    let scale = monitor.scale_factor().max(0.1);
    let work = monitor.work_area();
    let work_x = f64::from(work.position.x) / scale;
    let work_y = f64::from(work.position.y) / scale;
    let work_width = f64::from(work.size.width) / scale;
    let work_height = f64::from(work.size.height) / scale;
    if work_width <= 0.0 || work_height <= 0.0 {
        window.center()?;
        return Ok(());
    }

    let available_width = (work_width - FRAME_WIDTH).max(1.0);
    let available_height = (work_height - FRAME_HEIGHT).max(1.0);
    let width = (available_width * 0.98)
        .min(PREFERRED_WIDTH)
        .max(MIN_WIDTH.min(available_width));
    let height = (available_height * 0.98)
        .min(PREFERRED_HEIGHT)
        .max(MIN_HEIGHT.min(available_height));

    let outer_width = width + FRAME_WIDTH;
    let outer_height = height + FRAME_HEIGHT;
    let x = work_x + ((work_width - outer_width) / 2.0).max(0.0);
    let y = work_y + ((work_height - outer_height) / 2.0).max(0.0);

    window.set_size(LogicalSize::new(width, height))?;
    window.set_position(LogicalPosition::new(x, y))?;
    Ok(())
}

/// Reveal the main window after the frontend has mounted.
#[tauri::command]
fn show_main_window(window: WebviewWindow) {
    let _ = fit_and_center(&window);
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
                let _ = fit_and_center(&window);
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
