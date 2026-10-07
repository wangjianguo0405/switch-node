use crate::config::{cache_dir, AppConfig};
use crate::error::AppError;
use chrono::{NaiveDate, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    match fetch_index_body(config).await {
        Ok(body) => {
            let entries: Vec<IndexEntry> = serde_json::from_str(&body)?;
            let versions = classify_versions(config, &entries);

            // Cache the classified list and the raw index side by side
            if let Some(parent) = cache_file.parent() {
                fs::create_dir_all(parent)?;
            }
            if let Ok(json) = serde_json::to_string(&versions) {
                let _ = fs::write(&cache_file, json);
            }
            write_raw_cache(&body);

            info!("Fetched {} remote versions", versions.len());
            Ok(versions)
        }
        Err(e) => {
            warn!("All mirrors failed: {}", e);

            // Cache fallback for offline use
            if cache_file.exists() {
                if let Ok(content) = fs::read_to_string(&cache_file) {
                    if let Ok(versions) = serde_json::from_str::<Vec<RemoteVersion>>(&content) {
                        info!("Using cached fallback index (offline mode)");
                        return Ok(versions);
                    }
                }
            }

            Err(e)
        }
    }
}

/// Fetch the raw index.json body — primary mirror first, then taobao fallback
async fn fetch_index_body(config: &AppConfig) -> Result<String, AppError> {
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

        match client.get(url).send().await {
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
                info!("Fetched index body from {}", url);
                return Ok(body);
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

    Err(last_error.unwrap_or_else(|| {
        AppError::Offline("离线模式，无法获取远程版本列表".to_string())
    }))
}

/// Path of the cached raw index.json body
fn raw_cache_path() -> PathBuf {
    cache_dir().join("index-raw.json")
}

fn read_raw_cache() -> Option<Vec<IndexEntry>> {
    let content = fs::read_to_string(raw_cache_path()).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_raw_cache(body: &str) {
    let path = raw_cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, body);
}

/// Look up metadata for one exact version, including patches that are not the
/// latest of their major line — the sidebar only lists the latest per line, so
/// anything else installed locally would otherwise have no metadata at all.
pub async fn fetch_version_detail(
    config: &AppConfig,
    version: &str,
) -> Result<RemoteVersion, AppError> {
    let target = version.trim_start_matches('v');

    let detail = match read_raw_cache()
        .and_then(|entries| build_detail(config, &entries, target))
    {
        Some(detail) => detail,
        None => {
            // Raw index missing or predates this version — refresh once and retry
            let body = fetch_index_body(config).await?;
            let entries: Vec<IndexEntry> = serde_json::from_str(&body)?;
            write_raw_cache(&body);

            build_detail(config, &entries, target)
                .ok_or_else(|| AppError::VersionNotFound(target.to_string()))?
        }
    };

    info!("Resolved metadata for v{}", detail.version);
    Ok(detail)
}

/// Entries of one major version line, with the line's first release date
struct MajorGroup<'a> {
    first_date: NaiveDate,
    entries: Vec<&'a IndexEntry>,
}

/// Filter raw entries by architecture and group them by major version
fn group_by_major<'a>(
    config: &AppConfig,
    entries: &'a [IndexEntry],
) -> HashMap<u64, MajorGroup<'a>> {
    let arch_filter = format!("win-{}-zip", config.architecture);
    let mut groups: HashMap<u64, MajorGroup<'a>> = HashMap::new();

    for entry in entries {
        if !entry.files.iter().any(|f| f == &arch_filter) {
            continue;
        }
        let Ok(sv) = semver::Version::parse(entry.version.trim_start_matches('v')) else {
            continue;
        };
        let date = NaiveDate::parse_from_str(&entry.date, "%Y-%m-%d").unwrap_or_default();

        let group = groups.entry(sv.major).or_insert_with(|| MajorGroup {
            first_date: date,
            entries: Vec::new(),
        });
        if date < group.first_date {
            group.first_date = date;
        }
        group.entries.push(entry);
    }

    groups
}

fn status_for_major(
    major: u64,
    first_date: NaiveDate,
    latest_odd_major: Option<u64>,
    now: NaiveDate,
) -> VersionStatus {
    let months_since_first = (now - first_date).num_days() as f64 / 30.44;

    // Even versions follow the LTS lifecycle; odd versions never enter LTS.
    // The major number is used, NOT the lts field, because even versions may
    // have lts:false before the codename is assigned.
    if major % 2 == 0 {
        if months_since_first < 6.0 {
            VersionStatus::Current
        } else if months_since_first < 18.0 {
            VersionStatus::ActiveLts
        } else if months_since_first < 36.0 {
            VersionStatus::MaintenanceLts
        } else {
            VersionStatus::Eol
        }
    } else if latest_odd_major == Some(major) && months_since_first < 6.0 {
        VersionStatus::Current
    } else {
        VersionStatus::Eol
    }
}

fn build_remote_version(
    entry: &IndexEntry,
    status: VersionStatus,
    first_date: NaiveDate,
    is_latest: bool,
    is_latest_lts: bool,
) -> RemoteVersion {
    let lts_value = match &entry.lts {
        serde_json::Value::String(s) => Some(s.clone()),
        _ => None,
    };

    RemoteVersion {
        version: entry.version.trim_start_matches('v').to_string(),
        date: entry.date.clone(),
        lts: lts_value,
        npm: entry.npm.clone().unwrap_or_default(),
        v8: entry.v8.clone().unwrap_or_default(),
        openssl: entry.openssl.clone().unwrap_or_default(),
        files: entry.files.clone(),
        security: entry.security,
        status: status.clone(),
        status_label: VersionStatus::label(&status).to_string(),
        status_color: VersionStatus::color(&status).to_string(),
        status_description: VersionStatus::description(&status, Some(first_date)),
        is_latest,
        is_latest_lts,
        download_size_mb: 28.0, // ~28MB for a typical win-x64 zip
    }
}

/// Build the metadata for one exact version out of the raw index
fn build_detail(config: &AppConfig, entries: &[IndexEntry], target: &str) -> Option<RemoteVersion> {
    let groups = group_by_major(config, entries);
    let latest_odd_major = groups.keys().copied().filter(|m| m % 2 == 1).max();
    let now = Utc::now().date_naive();

    for (major, group) in &groups {
        let Some(entry) = group
            .entries
            .iter()
            .find(|e| e.version.trim_start_matches('v') == target)
        else {
            continue;
        };

        let status = status_for_major(*major, group.first_date, latest_odd_major, now);
        // A version reaching this lookup is absent from the classified list,
        // so it is never a line's latest patch, nor the overall latest / latest LTS.
        return Some(build_remote_version(entry, status, group.first_date, false, false));
    }

    None
}

/// Classify raw index entries into categorized RemoteVersions
fn classify_versions(config: &AppConfig, entries: &[IndexEntry]) -> Vec<RemoteVersion> {
    let mut groups = group_by_major(config, entries);

    // Filter by minVersion (whole majors only, so per-major groups are unaffected)
    if let Ok(min_ver) = semver::Version::parse(&config.min_version) {
        groups.retain(|major, _| *major >= min_ver.major);
    }

    let latest_odd_major = groups.keys().copied().filter(|m| m % 2 == 1).max();
    let now = Utc::now().date_naive();

    // Note: EOL filtering is done on the frontend side. Always include every
    // version so the user can toggle visibility freely.
    let mut result: Vec<RemoteVersion> = groups
        .iter()
        .filter_map(|(major, group)| {
            // Only emit the latest patch version per major line (like nodejs.org)
            let latest_entry = group
                .entries
                .iter()
                .max_by_key(|e| {
                    semver::Version::parse(e.version.trim_start_matches('v'))
                        .unwrap_or_else(|_| semver::Version::new(0, 0, 0))
                })?;

            let status = status_for_major(*major, group.first_date, latest_odd_major, now);
            Some(build_remote_version(
                latest_entry,
                status,
                group.first_date,
                false, // is_latest / is_latest_lts computed below
                false,
            ))
        })
        .collect();

    // Sort by version (newest first)
    let fallback = semver::Version::new(0, 0, 0);
    result.sort_by(|a, b| {
        let a_ver = a.version.trim_start_matches('v');
        let b_ver = b.version.trim_start_matches('v');
        semver::Version::parse(b_ver)
            .unwrap_or(fallback.clone())
            .cmp(&semver::Version::parse(a_ver).unwrap_or(fallback.clone()))
    });

    // Mark "latest" versions
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

