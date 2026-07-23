mod commands;
mod config;
mod downloader;
mod error;
mod node_manager;
mod remote;
mod updater;

use chrono::Utc;
use log::info;
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct AppState {
    pub config: Mutex<config::AppConfig>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            // On second instance, focus the existing window
            info!("Second instance detected, focusing existing window");
        }))
        .setup(|app| {
            info!("🚀 switch-node starting up...");

            // Load config on startup
            let app_config = config::load_config()
                .unwrap_or_else(|e| {
                    info!("Could not load config: {}, using defaults", e);
                    config::default_config()
                });

            app.manage(AppState {
                config: Mutex::new(app_config),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_config,
            commands::scan_local_versions,
            commands::fetch_remote_versions,
            commands::download_version,
            commands::switch_version,
            commands::get_active_version,
            commands::remove_version,
            commands::open_release_notes,
            commands::get_app_info,
            commands::detect_system_node,
            // Update commands
            check_update,
            download_and_install,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ── Update commands ────────────────────────────────────────────

#[tauri::command]
async fn check_update(state: State<'_, AppState>) -> Result<updater::UpdateInfo, String> {
    let app_config = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.clone()
    };

    let info = updater::check_update(&app_config).await?;

    // Update last_check timestamp and save
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.last_check = Some(Utc::now().to_rfc3339());
        config::save_config(&config).map_err(|e| e.to_string())?;
    }

    Ok(info)
}

#[tauri::command]
fn download_and_install(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    download_url: String,
    asset_name: String,
) -> Result<(), String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .unwrap()
        .to_path_buf();

    let config = state.config.lock().map_err(|e| e.to_string())?;
    updater::download_and_install(
        &download_url,
        &asset_name,
        &exe_dir,
        config.github_token.as_deref(),
    )?;

    // Batch script launched — exit app
    app_handle.exit(0);
    Ok(())
}
