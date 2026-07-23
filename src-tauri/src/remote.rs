use crate::config::{cache_dir, AppConfig};
use crate::error::AppError;
use chrono::{NaiveDate, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteVersion {
    pub version: String,
    pub date: String,
    pub lts: Option<String>, // false -> None, codename -> Some
    pub npm: String,
    pub v8: String,
    pub openssl: String,
    pub files: Vec<String>,
    pub security: bool,
    pub status: VersionStatus,
    pub status_label: String,
    pub status_color: String,
    pub status_description: String,
    #[serde(rename = "isLatest")]
    pub is_latest: bool,
    #[serde(rename = "isLatestLts")]
    pub is_latest_lts: bool,
    pub download_size_mb: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VersionStatus {
    Current,
    ActiveLts,
    MaintenanceLts,
    Eol,
}

impl VersionStatus {
    fn label(&self) -> &str {
        match self {
            VersionStatus::Current => "Current",
            VersionStatus::ActiveLts => "LTS",
            VersionStatus::MaintenanceLts => "LTS",
            VersionStatus::Eol => "EOL",
        }
    }

    fn color(&self) -> &str {
        match self {
            VersionStatus::Current => "current",
            VersionStatus::ActiveLts => "current",
            VersionStatus::MaintenanceLts => "maintenance",
            VersionStatus::Eol => "eol",
        }
    }

    fn description(&self, first_release: Option<NaiveDate>) -> String {
        match self {
            VersionStatus::Current => {
                if let Some(date) = first_release {
                    let lts_date = date + chrono::Duration::days(183); // ~6 months
                    format!(
                        "将于 {} 进入 Active LTS",
                        lts_date.format("%Y-%m")
                    )
                } else {
                    "Current — 最新特性版本".to_string()
                }
            }
            VersionStatus::ActiveLts => "Active LTS — 积极维护中".to_string(),
            VersionStatus::MaintenanceLts => "Maintenance LTS — 仅修关键 bug".to_string(),
            VersionStatus::Eol => {
                if let Some(date) = first_release {
                    let eol_date = date + chrono::Duration::days(36 * 30); // ~36 months
                    format!("已于 {} 终止支持", eol_date.format("%Y-%m-%d"))
                } else {
                    "已终止支持".to_string()
                }
            }
        }
        .to_string()
    }
}

/// Raw entry from index.json
#[derive(Debug, Deserialize)]
struct IndexEntry {
    version: String,
    date: String,
    files: Vec<String>,
    lts: serde_json::Value, // false or string codename
    npm: Option<String>,
    v8: Option<String>,
    openssl: Option<String>,
    security: bool,
}

/// Fetch and parse remote version index
pub async fn fetch_remote_versions(
    config: &AppConfig,
    force: bool,
) -> Result<Vec<RemoteVersion>, AppError> {
    let cache_dir = cache_dir();
    let cache_file = cache_dir.join("index.json");

    // Check cache freshness
    if !force && cache_file.exists() {
        if let Ok(metadata) = fs::metadata(&cache_file) {
            if let Ok(modified) = metadata.modified() {
                let age = modified
                    .elapsed()
                    .unwrap_or_default()
                    .as_secs();
                let max_age = (config.update_interval_hours as u64) * 3600;

                if age < max_age {
                    info!(
                        "Using cached index.json (age: {}s, max: {}s)",
                        age, max_age
                    );
                    if let Ok(content) = fs::read_to_string(&cache_file) {
                        if let Ok(versions) = serde_json::from_str::<Vec<RemoteVersion>>(&content) {
                            return Ok(versions);
                        }
                    }
                }
            }
        }
    }

    // Fetch from remote (primary mirror, then taobao fallback)
    let primary_url = format!("{}/index.json", config.mirror.trim_end_matches('/'));
    let fallback_url = "https://npmmirror.com/mirrors/node/index.json";

    let client = reqwest::Client::builder()
        .user_agent("switch-node/1.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let urls = if config.mirror.contains("npmmirror") || config.mirror.contains("taobao") {
        vec![primary_url]
    } else {
        vec![primary_url, fallback_url.to_string()]
    };

    let mut last_error: Option<AppError> = None;

    for (i, url) in urls.iter().enumerate() {
        if i > 0 {
            info!("Trying fallback mirror: {}", url);
        } else {
            info!("Fetching remote index from: {}", url);
        }

        let response = client.get(url).send().await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    last_error = Some(AppError::Http(format!(
                        "HTTP {} from {}",
                        resp.status(),
                        url
                    )));
                    continue;
                }

                let body = resp.text().await?;
                let entries: Vec<IndexEntry> = serde_json::from_str(&body)?;

                let versions = classify_versions(config, &entries);

                // Cache the result
                if let Some(parent) = cache_file.parent() {
                    fs::create_dir_all(parent)?;
                }
                if let Ok(json) = serde_json::to_string(&versions) {
                    let _ = fs::write(&cache_file, json);
                }

                info!("Fetched {} remote versions from {}", versions.len(), url);
                return Ok(versions);
            }
            Err(e) => {
                warn!("Failed to fetch from {}: {}", url, e);
                last_error = Some(AppError::Network(format!(
                    "Failed to fetch from {}: {}",
                    url, e
                )));
            }
        }
    }

    // All URLs failed, try cache fallback
    warn!("All mirrors failed, trying cache fallback");

    if cache_file.exists() {
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(versions) = serde_json::from_str::<Vec<RemoteVersion>>(&content) {
                info!("Using cached fallback index (offline mode)");
                return Ok(versions);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        AppError::Offline("离线模式，无法获取远程版本列表".to_string())
    }))
}

/// Classify raw index entries into categorized RemoteVersions
fn classify_versions(config: &AppConfig, entries: &[IndexEntry]) -> Vec<RemoteVersion> {
    // Filter by architecture
    let arch_filter = format!("win-{}-zip", config.architecture);

    // Step 1: Filter entries that have the matching arch zip file
    let mut valid_entries: Vec<&IndexEntry> = entries
        .iter()
        .filter(|e| e.files.iter().any(|f| f == &arch_filter))
        .collect();

    // Filter by minVersion
    if !config.min_version.is_empty() {
        if let Ok(min_ver) = semver::Version::parse(&config.min_version) {
            let min_major = min_ver.major;
            valid_entries.retain(|e| {
                let v = e.version.trim_start_matches('v');
                if let Ok(sv) = semver::Version::parse(v) {
                    sv.major >= min_major
                } else {
                    true
                }
            });
        }
    }

    // Step 2: Group by major version, find first release date
    use std::collections::HashMap;
    let mut major_groups: HashMap<u64, (NaiveDate, Vec<&IndexEntry>)> = HashMap::new();

    for entry in &valid_entries {
        let v = entry.version.trim_start_matches('v');
        if let Ok(sv) = semver::Version::parse(v) {
            let major = sv.major;
            let date = NaiveDate::parse_from_str(&entry.date, "%Y-%m-%d").unwrap_or_default();

            let group = major_groups.entry(major).or_insert_with(|| (date, Vec::new()));
            if date < group.0 {
                group.0 = date;
            }
            group.1.push(entry);
        }
    }

    let now = Utc::now().date_naive();

    // Step 3: Determine Current / LTS / EOL status for each major group
    // Find the latest odd major (for "Current" odd version handling)
    let latest_odd_major = major_groups
        .iter()
        .filter(|(m, _)| *m % 2 == 1)
        .map(|(m, _)| *m)
        .max();

    let mut result: Vec<RemoteVersion> = Vec::new();

    for (major, (first_date, group_entries)) in &major_groups {
        let months_since_first = (now - *first_date).num_days() as f64 / 30.44;

        // Fallback version for comparison
        let fallback = semver::Version::new(0, 0, 0);

        // Find the latest entry in this group (with the lts value)
        let latest_entry = group_entries
            .iter()
            .max_by(|a, b| {
                let a_ver = a.version.trim_start_matches('v');
                let b_ver = b.version.trim_start_matches('v');
                semver::Version::parse(a_ver)
                    .unwrap_or(fallback.clone())
                    .cmp(&semver::Version::parse(b_ver).unwrap_or(fallback.clone()))
            })
            .unwrap();

        // Determine status based on whether the major version is even (LTS-eligible)
        // or odd (never enters LTS). We use the major number, NOT the lts field,
        // because even versions may have lts:false before the codename is assigned.
        let is_even = major % 2 == 0;

        let status = if is_even {
            // Even version → follows the LTS lifecycle
            if months_since_first < 6.0 {
                VersionStatus::Current
            } else if months_since_first < 18.0 {
                VersionStatus::ActiveLts
            } else if months_since_first < 36.0 {
                VersionStatus::MaintenanceLts
            } else {
                VersionStatus::Eol
            }
        } else {
            // Odd version → Current only if it's the latest odd major and < 6 months
            let is_latest_odd = latest_odd_major.map(|m| m == *major).unwrap_or(false);
            if is_latest_odd && months_since_first < 6.0 {
                VersionStatus::Current
            } else {
                VersionStatus::Eol
            }
        };

        // Note: EOL filtering is now done on the frontend side.
        // Always include all versions so the user can toggle visibility freely.

        let lts_value = match &latest_entry.lts {
            serde_json::Value::String(s) => Some(s.clone()),
            _ => None,
        };

        // For the description, pass the first release date
        let description = VersionStatus::description(&status, Some(*first_date));

        // Estimate download size (~28MB for typical win-x64 zip)
        let download_size_mb = 28.0;

        // Only emit the latest patch version per major line (like nodejs.org)
        let clean_ver = latest_entry.version.trim_start_matches('v').to_string();
        result.push(RemoteVersion {
            version: clean_ver,
            date: latest_entry.date.clone(),
            lts: lts_value.clone(),
            npm: latest_entry.npm.clone().unwrap_or_default(),
            v8: latest_entry.v8.clone().unwrap_or_default(),
            openssl: latest_entry.openssl.clone().unwrap_or_default(),
            files: latest_entry.files.clone(),
            security: latest_entry.security,
            status: status.clone(),
            status_label: VersionStatus::label(&status).to_string(),
            status_color: VersionStatus::color(&status).to_string(),
            status_description: description.clone(),
            is_latest: false,    // Will be computed below
            is_latest_lts: false, // Will be computed below
            download_size_mb,
        });
    }

    // Step 4: Sort by version (newest first)
    let fallback = semver::Version::new(0, 0, 0);
    result.sort_by(|a, b| {
        let a_ver = a.version.trim_start_matches('v');
        let b_ver = b.version.trim_start_matches('v');
        semver::Version::parse(b_ver)
            .unwrap_or(fallback.clone())
            .cmp(&semver::Version::parse(a_ver).unwrap_or(fallback.clone()))
    });

    // Step 5: Mark "latest" versions
    // Latest overall
    if let Some(latest) = result.first() {
        let latest_ver = latest.version.clone();
        if let Some(first) = result.iter_mut().find(|v| v.version == latest_ver) {
            first.is_latest = true;
        }
    }

    // Latest LTS (among non-EOL)
    if let Some(latest_lts) = result
        .iter()
        .filter(|v| v.status != VersionStatus::Eol)
        .next()
    {
        let version = latest_lts.version.clone();
        if let Some(v) = result.iter_mut().find(|v| v.version == version) {
            v.is_latest_lts = true;
        }
    }

    result
}

/// Get cache file path
pub fn get_cache_path() -> PathBuf {
    cache_dir().join("index.json")
}
