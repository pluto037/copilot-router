use tauri::Manager;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod auth;
pub mod commands;
pub mod proxy;
pub mod state;
pub mod usage;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "copilot_router=info,tower_http=info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let _ = &app_handle; // reserved for future IPC use

            // Initialize state
            let db_path = app.path().app_data_dir()
                .expect("Failed to get app data dir")
                .join("copilot-router.db");

            let state = tauri::async_runtime::block_on(async {
                AppState::new(db_path).await.expect("Failed to initialize app state")
            });
            let initial_config = state.config.clone();

            let state = Arc::new(Mutex::new(state));
            app.manage(state.clone());

            tauri::async_runtime::spawn(async move {
                if let Err(e) = commands::sync_claude_code_proxy_settings(&initial_config).await {
                    tracing::warn!("Failed to sync Claude settings on startup: {}", e);
                }
            });

            // Start proxy server in background
            let state_for_proxy = state.clone();
            tauri::async_runtime::spawn(async move {
                let port = {
                    let s = state_for_proxy.lock().await;
                    s.config.proxy_port
                };
                if let Err(e) = proxy::server::start(state_for_proxy, port).await {
                    tracing::error!("Proxy server error: {}", e);
                }
            });

            // Setup system tray
            #[cfg(desktop)]
            {
                let tray = tauri::tray::TrayIconBuilder::new()
                    .icon(app.default_window_icon().unwrap().clone())
                    .tooltip("Copilot Router")
                    .on_tray_icon_event(|tray, event| {
                        if let tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;

                let _ = tray;
            }

            // Start token refresh loop
            let state_for_refresh = state.clone();
            tauri::async_runtime::spawn(async move {
                auth::refresher::start_refresh_loop(state_for_refresh).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_proxy_status,
            commands::get_token_status,
            commands::get_claude_takeover_status,
            commands::repair_claude_takeover,
            commands::get_usage_stats,
            commands::get_copilot_usage_overview,
            commands::get_recent_logs,
            commands::get_config,
            commands::save_config,
            commands::test_model_mapping,
            commands::start_proxy,
            commands::stop_proxy,
            commands::refresh_token,
            commands::auto_detect_token,
            commands::request_github_device_code,
            commands::wait_github_device_token,
            commands::copy_to_clipboard,
            commands::clear_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
