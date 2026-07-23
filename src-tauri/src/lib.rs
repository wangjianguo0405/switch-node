mod commands;
mod config;
mod downloader;
mod error;
mod node_manager;
mod remote;
mod updater;

use log::info;
use std::sync::Mutex;
use tauri::Manager;

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
            get_update_config,
            save_update_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ── Update commands ────────────────────────────────────────────

#[tauri::command]
fn check_update() -> Result<updater::UpdateInfo, String> {
    let config = updater::load_update_config();
    let info = updater::check_update(&config)?;

    // Update last_check timestamp
    let mut updated_config = config;
    updated_config.last_check = Some(chrono::Utc::now().to_rfc3339());
    let _ = updater::save_update_config(&updated_config);

    Ok(info)
}

#[tauri::command]
fn download_and_install(
    app_handle: tauri::AppHandle,
    download_url: String,
    asset_name: String,
) -> Result<(), String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .unwrap()
        .to_path_buf();

    let config = updater::load_update_config();
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

#[tauri::command]
fn get_update_config() -> Result<updater::UpdateConfig, String> {
    Ok(updater::load_update_config())
}

#[tauri::command]
fn save_update_config(config: updater::UpdateConfig) -> Result<(), String> {
    updater::save_update_config(&config)
}
