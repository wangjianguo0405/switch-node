use crate::config::{self, AppConfig};
use crate::downloader;
use crate::error::AppError;
use crate::node_manager::{self, LocalVersion, SystemNodeInfo};
use crate::remote::{self, RemoteVersion};
use crate::AppState;
use log::info;
use serde::Serialize;
use tauri::{command, AppHandle, Emitter, State};

type ConfigState<'a> = State<'a, AppState>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub is_admin: bool,
}

/// Get current configuration
#[command]
pub fn get_config(state: ConfigState<'_>) -> Result<AppConfig, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

/// Update configuration (partial merge)
#[command]
pub fn set_config(state: ConfigState<'_>, partial: serde_json::Value) -> Result<AppConfig, String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;

    // Remember old npmMirror before merge
    let old_npm_mirror = config.npm_mirror.clone();

    // Merge partial JSON into current config
    if let Ok(partial_config) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(partial.clone()) {
        // Convert current config to Value, merge, convert back
        let mut current_value = serde_json::to_value(&*config).unwrap_or_default();
        if let Some(current_obj) = current_value.as_object_mut() {
            for (key, value) in partial_config {
                current_obj.insert(key, value);
            }
        }
        if let Ok(new_config) = serde_json::from_value::<AppConfig>(current_value) {
            *config = new_config;
        }
    }

    // Save to disk
    config::save_config(&config).map_err(|e| e.to_string())?;
    info!("Configuration saved");

    // Apply npmMirror if changed
    if config.npm_mirror != old_npm_mirror {
        let mirror = config.npm_mirror.clone();
        info!("npmMirror changed, running: npm config set registry {}", mirror);
        match std::process::Command::new("npm")
            .args(["config", "set", "registry", &mirror])
            .output()
        {
            Ok(output) if output.status.success() => {
                info!("npm config set registry succeeded");
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                info!("npm config set registry failed (npm may not be in PATH): {}", stderr.trim());
            }
            Err(e) => {
                info!("npm config set registry failed (npm may not be installed): {}", e);
            }
        }
    }

    Ok(config.clone())
}

/// Scan local installed versions
#[command]
pub fn scan_local_versions(state: ConfigState<'_>) -> Result<Vec<LocalVersion>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let versions = node_manager::scan_local_versions(&config);
    info!("Scanned {} local versions", versions.len());
    Ok(versions)
}

/// Fetch remote versions from index.json
#[command]
pub async fn fetch_remote_versions(
    state: ConfigState<'_>,
    force: bool,
) -> Result<Vec<RemoteVersion>, String> {
    let config = {
        let guard = state.config.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };
    remote::fetch_remote_versions(&config, force)
        .await
        .map_err(|e| match e {
            AppError::Offline(_) => {
                info!("Offline mode, returning empty (will use cache in frontend)");
                "OFFLINE".to_string()
            }
            other => other.to_string(),
        })
}

/// Download and install a version with progress events
#[command]
pub async fn download_version(
    state: ConfigState<'_>,
    app_handle: AppHandle,
    version: String,
) -> Result<String, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();

    info!("Starting download of version {}", version);

    let app_handle_clone = app_handle.clone();
    let version_clone = version.clone();

    downloader::download_and_install(&config, &version, move |progress| {
        let _ = app_handle_clone.emit("download-progress", progress);
    })
    .await
    .map_err(|e| e.to_string())?;

    // Emit completion event
    let _ = app_handle.emit(
        "download-complete",
        serde_json::json!({ "version": version_clone }),
    );

    info!("Download and install complete for {}", version_clone);
    Ok(version_clone)
}

/// Switch to a specific version
#[command]
pub fn switch_version(
    state: ConfigState<'_>,
    app_handle: AppHandle,
    version: String,
) -> Result<String, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;

    info!("Switching to version {}", version);

    match node_manager::switch_version(&config, &version) {
        Ok(()) => {
            let _ = app_handle.emit(
                "version-switched",
                serde_json::json!({
                    "version": version,
                }),
            );
            info!("Successfully switched to {}", version);
            Ok(version)
        }
        Err(e) => {
            let err_msg = e.to_string();
            let _ = app_handle.emit(
                "switch-error",
                serde_json::json!({ "error": err_msg }),
            );
            Err(err_msg)
        }
    }
}

/// Get the currently active version
#[command]
pub fn get_active_version(state: ConfigState<'_>) -> Result<Option<String>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(node_manager::get_active_version(&config))
}

/// Remove a specific version
#[command]
pub fn remove_version(state: ConfigState<'_>, version: String) -> Result<(), String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    node_manager::remove_version(&config, &version).map_err(|e| e.to_string())?;
    info!("Removed version {}", version);
    Ok(())
}

/// Open release notes in browser
#[command]
pub fn open_release_notes(version: String) -> Result<(), String> {
    let clean_version = version.trim_start_matches('v');
    let major = clean_version
        .split('.')
        .next()
        .unwrap_or("22");
    let url = format!(
        "https://github.com/nodejs/node/blob/main/doc/changelogs/CHANGELOG_V{}.md",
        major
    );
    open::that(&url).map_err(|e| e.to_string())?;
    info!("Opened release notes for v{}", major);
    Ok(())
}

/// Detect system-installed Node.js (outside managed directory)
#[command]
pub fn detect_system_node(state: ConfigState<'_>) -> Result<Option<SystemNodeInfo>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(node_manager::detect_system_node(&config))
}

/// Get application info
#[command]
pub fn get_app_info() -> Result<AppInfo, String> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        is_admin: node_manager::is_admin(),
    })
}
