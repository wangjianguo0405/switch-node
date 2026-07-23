use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    // Paths
    #[serde(default = "default_node_root")]
    pub node_root: String,
    #[serde(default = "default_symlink_name")]
    pub symlink_name: String,

    // Download
    #[serde(default = "default_mirror")]
    pub mirror: String,
    #[serde(default = "default_mirror_name")]
    pub mirror_name: String,
    #[serde(default = "default_architecture")]
    pub architecture: String,
    #[serde(default)]
    pub keep_downloads: bool,
    #[serde(default)]
    pub download_dir: String,

    // Version policy
    #[serde(default = "default_min_version")]
    pub min_version: String,
    #[serde(default = "default_show_eol")]
    pub show_eol: bool,
    #[serde(default)]
    pub prerelease: bool,

    // Behavior
    #[serde(default = "default_auto_refresh_env")]
    pub auto_refresh_env: bool,
    #[serde(default = "default_check_updates")]
    pub check_updates: bool,
    #[serde(default = "default_update_interval_hours")]
    pub update_interval_hours: u32,

    // npm
    #[serde(default = "default_npm_mirror")]
    pub npm_mirror: String,

    // UI
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,

}

fn default_node_root() -> String {
    "D:\\Program Files\\nodejs".to_string()
}
fn default_symlink_name() -> String {
    "current".to_string()
}
fn default_mirror() -> String {
    "https://nodejs.org/dist".to_string()
}
fn default_mirror_name() -> String {
    "official".to_string()
}
fn default_architecture() -> String {
    "x64".to_string()
}
fn default_min_version() -> String {
    "18.0.0".to_string()
}
fn default_show_eol() -> bool {
    false
}
fn default_auto_refresh_env() -> bool {
    true
}
fn default_check_updates() -> bool {
    true
}
fn default_update_interval_hours() -> u32 {
    1
}
fn default_npm_mirror() -> String {
    "https://registry.npmjs.org".to_string()
}
fn default_language() -> String {
    "zh-CN".to_string()
}
fn default_theme() -> String {
    "system".to_string()
}
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            node_root: default_node_root(),
            symlink_name: default_symlink_name(),
            mirror: default_mirror(),
            mirror_name: default_mirror_name(),
            architecture: default_architecture(),
            keep_downloads: false,
            download_dir: String::new(),
            min_version: default_min_version(),
            show_eol: default_show_eol(),
            prerelease: false,
            auto_refresh_env: default_auto_refresh_env(),
            check_updates: default_check_updates(),
            update_interval_hours: default_update_interval_hours(),
            npm_mirror: default_npm_mirror(),
            language: default_language(),
            theme: default_theme(),
        }
    }
}

/// Get the directory containing the executable (portable app root).
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Detect config file path — always next to the executable (portable mode).
pub fn detect_config_path() -> PathBuf {
    exe_dir().join("config.json")
}

/// Detect config format by trying each extension in the exe directory.
pub fn detect_config_path_with_format() -> (PathBuf, ConfigFormat) {
    let base_dir = exe_dir();

    // Priority: json > toml > ini
    for (ext, format) in &[
        ("json", ConfigFormat::Json),
        ("toml", ConfigFormat::Toml),
        ("ini", ConfigFormat::Ini),
    ] {
        let path = base_dir.join(format!("config.{}", ext));
        if path.exists() {
            return (path, *format);
        }
    }

    // Default: config.json
    (base_dir.join("config.json"), ConfigFormat::Json)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigFormat {
    Json,
    Toml,
    Ini,
}

/// Get the config/cache/logs root directory — exe directory for portable mode.
pub fn config_dir() -> PathBuf {
    exe_dir()
}

/// Get the cache directory for remote index.
pub fn cache_dir() -> PathBuf {
    config_dir().join("cache")
}

/// Get the logs directory.
pub fn logs_dir() -> PathBuf {
    config_dir().join("logs")
}

/// Load configuration from the detected path
pub fn load_config() -> Result<AppConfig, AppError> {
    let (config_path, format) = detect_config_path_with_format();

    if !config_path.exists() {
        // First run: generate default config
        let default_config = AppConfig::default();
        save_config_to_path(&config_path, &default_config, format)?;
        return Ok(default_config);
    }

    let content = fs::read_to_string(&config_path).map_err(|e| {
        AppError::Config(format!("Cannot read config file {:?}: {}", config_path, e))
    })?;

    let config = match format {
        ConfigFormat::Json => serde_json::from_str(&content)?,
        ConfigFormat::Toml => toml::from_str(&content).map_err(AppError::from)?,
        ConfigFormat::Ini => parse_ini_config(&content)?,
    };

    Ok(config)
}

/// Save configuration to the detected path
pub fn save_config(config: &AppConfig) -> Result<(), AppError> {
    let (config_path, format) = detect_config_path_with_format();

    // Ensure directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    save_config_to_path(&config_path, config, format)
}

fn save_config_to_path(path: &Path, config: &AppConfig, format: ConfigFormat) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = match format {
        ConfigFormat::Json => serde_json::to_string_pretty(config)?,
        ConfigFormat::Toml => toml::to_string_pretty(config).map_err(AppError::from)?,
        ConfigFormat::Ini => serialize_ini_config(config)?,
    };

    fs::write(path, content)?;
    Ok(())
}

/// Parse .ini format into AppConfig
fn parse_ini_config(content: &str) -> Result<AppConfig, AppError> {
    use std::collections::HashMap;
    let ini: HashMap<String, HashMap<String, String>> =
        serde_ini::from_str(content).map_err(|e| AppError::Ini(e.to_string()))?;

    // Get values from the default (unnamed) section
    let section = ini.get("").or_else(|| ini.values().next());

    let get_val = |key: &str, default: &str| -> String {
        section
            .and_then(|s| s.get(key))
            .cloned()
            .unwrap_or_else(|| default.to_string())
    };

    let get_bool = |key: &str, default: bool| -> bool {
        section
            .and_then(|s| s.get(key))
            .map(|v| v == "true" || v == "1" || v == "yes")
            .unwrap_or(default)
    };

    let get_u32 = |key: &str, default: u32| -> u32 {
        section
            .and_then(|s| s.get(key))
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };

    Ok(AppConfig {
        node_root: get_val("nodeRoot", &default_node_root()),
        symlink_name: get_val("symlinkName", &default_symlink_name()),
        mirror: get_val("mirror", &default_mirror()),
        mirror_name: get_val("mirrorName", &default_mirror_name()),
        architecture: get_val("architecture", &default_architecture()),
        keep_downloads: get_bool("keepDownloads", false),
        download_dir: get_val("downloadDir", ""),
        min_version: get_val("minVersion", &default_min_version()),
        show_eol: get_bool("showEOL", true),
        prerelease: get_bool("prerelease", false),
        auto_refresh_env: get_bool("autoRefreshEnv", true),
        check_updates: get_bool("checkUpdates", true),
        update_interval_hours: get_u32("updateIntervalHours", 1),
        npm_mirror: get_val("npmMirror", &default_npm_mirror()),
        language: get_val("language", &default_language()),
        theme: get_val("theme", &default_theme()),
    })
}

/// Serialize AppConfig to INI format
fn serialize_ini_config(config: &AppConfig) -> Result<String, AppError> {
    let mut lines = Vec::new();
    lines.push("[switch-node]".to_string());
    lines.push(format!("nodeRoot={}", config.node_root));
    lines.push(format!("symlinkName={}", config.symlink_name));
    lines.push(format!("mirror={}", config.mirror));
    lines.push(format!("mirrorName={}", config.mirror_name));
    lines.push(format!("architecture={}", config.architecture));
    lines.push(format!("keepDownloads={}", config.keep_downloads));
    lines.push(format!("downloadDir={}", config.download_dir));
    lines.push(format!("minVersion={}", config.min_version));
    lines.push(format!("showEOL={}", config.show_eol));
    lines.push(format!("prerelease={}", config.prerelease));
    lines.push(format!("autoRefreshEnv={}", config.auto_refresh_env));
    lines.push(format!("checkUpdates={}", config.check_updates));
    lines.push(format!(
        "updateIntervalHours={}",
        config.update_interval_hours
    ));
    lines.push(format!("npmMirror={}", config.npm_mirror));
    lines.push(format!("language={}", config.language));
    lines.push(format!("theme={}", config.theme));
    Ok(lines.join("\n") + "\n")
}

pub fn default_config() -> AppConfig {
    AppConfig::default()
}
