use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

// ── Update Info ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: String,
    pub download_url: String,
    pub asset_name: String,
    pub asset_size: u64,
}

// ── GitHub API types ───────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    #[allow(dead_code)]
    prerelease: bool,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubAsset {
    name: String,
    #[allow(dead_code)]
    browser_download_url: String,
    url: String,
    size: u64,
}

// ── Version ────────────────────────────────────────────────────

pub fn get_current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ── GitHub API helpers (async, using reqwest) ─────────────────

fn build_headers(token: Option<&str>) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", "application/vnd.github.v3+json".parse().unwrap());
    headers.insert("User-Agent", "SwitchNode-Updater/1.0".parse().unwrap());
    if let Some(t) = token {
        if !t.is_empty() {
            headers.insert(
                "Authorization",
                format!("Bearer {}", t).parse().unwrap(),
            );
        }
    }
    headers
}

async fn make_request_async(url: &str, token: Option<&str>) -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .headers(build_headers(token))
        .send()
        .await
        .map_err(|e| format!("GitHub API request failed: {}", e))?;

    let status = resp.status();

    if status == reqwest::StatusCode::NOT_FOUND {
        return Err("NOT_FOUND".to_string());
    }
    if status == reqwest::StatusCode::FORBIDDEN {
        return Err(
            "GitHub API rate limit exceeded. Please wait and try again, or set a GitHub token."
                .to_string(),
        );
    }
    if !status.is_success() {
        return Err(format!("GitHub API error: HTTP {}", status));
    }

    resp.text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))
}

fn parse_tag_semver(tag: &str) -> semver::Version {
    semver::Version::parse(tag.trim_start_matches('v')).unwrap_or(semver::Version::new(0, 0, 0))
}

async fn try_get_latest(owner: &str, repo: &str, token: Option<&str>) -> Result<GitHubRelease, String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        owner, repo
    );
    let body = make_request_async(&url, token).await?;
    serde_json::from_str::<GitHubRelease>(&body)
        .map_err(|e| format!("JSON parse error: {}", e))
}

async fn fetch_release_list(
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> Result<GitHubRelease, String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases?per_page=20",
        owner, repo
    );
    let body = make_request_async(&url, token).await?;

    let mut releases: Vec<GitHubRelease> =
        serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {}", e))?;

    if releases.is_empty() {
        return Err("No releases found".to_string());
    }

    releases.sort_by(|a, b| {
        let va = parse_tag_semver(&a.tag_name);
        let vb = parse_tag_semver(&b.tag_name);
        vb.cmp(&va)
    });

    for r in &releases {
        if !r.assets.is_empty() {
            return Ok(r.clone());
        }
    }

    Ok(releases.into_iter().next().unwrap())
}

async fn fetch_latest_release(
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> Result<GitHubRelease, String> {
    match try_get_latest(owner, repo, token).await {
        Ok(release) => Ok(release),
        Err(_) => fetch_release_list(owner, repo, token).await,
    }
}

// ── Asset matching ─────────────────────────────────────────────

fn find_windows_asset(release: &GitHubRelease) -> Option<&GitHubAsset> {
    release
        .assets
        .iter()
        .find(|a| {
            let name = a.name.to_lowercase();
            name.contains("windows")
                && (name.contains("x64") || name.contains("x86_64"))
                && (name.ends_with(".exe") || name.ends_with(".zip"))
        })
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|a| a.name.ends_with(".exe") || a.name.ends_with(".zip"))
        })
}

// ── Check update ───────────────────────────────────────────────

pub async fn check_update(config: &AppConfig) -> Result<UpdateInfo, String> {
    if config.github_owner.is_empty() || config.github_repo.is_empty() {
        return Err(
            "Update repository not configured. Please set Owner/Repo in settings.".to_string(),
        );
    }

    let current = get_current_version();
    let release = match fetch_latest_release(
        &config.github_owner,
        &config.github_repo,
        config.github_token.as_deref(),
    ).await {
        Ok(r) => r,
        Err(e) => {
            // "No releases found" means the repo exists but has no releases yet — not an error
            if e.contains("No releases found") {
                return Ok(UpdateInfo {
                    has_update: false,
                    current_version: current.clone(),
                    latest_version: current,
                    release_notes: String::new(),
                    download_url: String::new(),
                    asset_name: String::new(),
                    asset_size: 0,
                });
            }
            return Err(e);
        }
    };

    let latest_tag = release.tag_name.trim_start_matches('v').to_string();

    let has_update = match (
        semver::Version::parse(&current),
        semver::Version::parse(&latest_tag),
    ) {
        (Ok(cur), Ok(latest)) => latest > cur,
        _ => latest_tag != current,
    };

    let asset = find_windows_asset(&release);

    Ok(UpdateInfo {
        has_update,
        current_version: current,
        latest_version: latest_tag,
        release_notes: release.body.clone().unwrap_or_default(),
        download_url: asset.map(|a| a.url.clone()).unwrap_or_default(),
        asset_name: asset.map(|a| a.name.clone()).unwrap_or_default(),
        asset_size: asset.map(|a| a.size).unwrap_or(0),
    })
}

// ── Download and install ───────────────────────────────────────

pub fn download_and_install(
    download_url: &str,
    asset_name: &str,
    app_dir: &Path,
    token: Option<&str>,
) -> Result<(), String> {
    let temp_dir = std::env::temp_dir().join("switch-node_update");
    let new_dir = temp_dir.join("new");

    if new_dir.exists() {
        let _ = std::fs::remove_dir_all(&new_dir);
    }
    std::fs::create_dir_all(&new_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    let is_exe = asset_name.to_lowercase().ends_with(".exe");
    let is_api_url = download_url.contains("api.github.com");

    let download_name = if is_exe {
        "switch-node.exe".to_string()
    } else {
        "update.zip".to_string()
    };
    let download_path = temp_dir.join(&download_name);

    // ── Download ──
    log::info!(
        "[UPDATER] Downloading ({}): {}",
        if is_exe { "exe" } else { "zip" },
        download_url
    );

    let mut dl_req = ureq::get(download_url).header("User-Agent", "SwitchNode-Updater/1.0");

    if is_api_url {
        dl_req = dl_req.header("Accept", "application/octet-stream");
    }

    if let Some(t) = token {
        if !t.is_empty() {
            dl_req = dl_req.header("Authorization", &format!("Bearer {}", t));
        }
    }

    let resp = dl_req
        .call()
        .map_err(|e| format!("Download failed: {}", e))?;

    let total = resp
        .headers()
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());

    let mut reader = resp.into_body().into_reader();
    let mut downloaded: u64 = 0;
    let mut buf = [0u8; 8192];
    let mut file = std::fs::File::create(&download_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("Download read error: {}", e))?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buf[..n])
            .map_err(|e| format!("File write error: {}", e))?;
        downloaded += n as u64;
        if let Some(total) = total {
            let pct = (downloaded * 100) / total;
            if pct % 10 == 0 {
                log::info!("[UPDATER] Download progress: {}%", pct);
            }
        }
    }
    drop(file);
    log::info!("[UPDATER] Downloaded {} bytes", downloaded);

    if is_exe {
        let dest = new_dir.join("switch-node.exe");
        std::fs::rename(&download_path, &dest)
            .or_else(|_| std::fs::copy(&download_path, &dest).map(|_| ()))
            .map_err(|e| format!("Failed to move file: {}", e))?;
    } else {
        log::info!("[UPDATER] Extracting to: {}", new_dir.display());
        let zip_file = std::fs::File::open(&download_path)
            .map_err(|e| format!("Failed to open zip: {}", e))?;
        let mut archive =
            zip::ZipArchive::new(zip_file).map_err(|e| format!("Failed to read zip: {}", e))?;

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;
            let name = entry.name().to_string();

            if name.ends_with('/') || name.starts_with("__MACOSX") || name.contains("/.") {
                continue;
            }

            let relative = if let Some(pos) = name.find('/') {
                &name[pos + 1..]
            } else {
                &name
            };

            if relative.is_empty() {
                continue;
            }

            let out_path = new_dir.join(relative);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create dir: {}", e))?;
            }

            let mut out_file = std::fs::File::create(&out_path)
                .map_err(|e| format!("Failed to create file {}: {}", out_path.display(), e))?;
            std::io::copy(&mut entry, &mut out_file)
                .map_err(|e| format!("Extract error {}: {}", out_path.display(), e))?;
        }

        let _ = std::fs::remove_file(&download_path);
        log::info!("[UPDATER] Extraction complete");
    }

    spawn_update_script(&new_dir, app_dir, is_exe)?;

    Ok(())
}

// ── Batch script generation ────────────────────────────────────

fn spawn_update_script(new_dir: &Path, app_dir: &Path, is_exe: bool) -> Result<(), String> {
    let script_dir = new_dir
        .parent()
        .expect("new_dir should have a parent (temp/switch-node_update)");

    let script_path = script_dir.join("updater.bat");
    let app_dir_str = app_dir.to_string_lossy().to_string();
    let new_dir_str = new_dir.to_string_lossy().to_string();

    let exe_name = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "switch-node.exe".to_string());

    let script = if is_exe {
        format!(
            r#"@echo off
echo [Updater] Waiting for app to exit...
timeout /t 3 /nobreak >nul

rem Find downloaded exe (name may differ)
set SRC=
for %%f in ("{new}\*.exe") do set SRC=%%f
if "%SRC%"=="" (
    echo [Updater] FAILED - no exe found
    rmdir /S /Q "{new}"
    del "%~f0"
    pause
    exit /b 1
)

echo [Updater] Source: %SRC%
echo [Updater] Copying...
copy /Y "%SRC%" "{app}\{exe}"

if %errorlevel% neq 0 (
    echo [Updater] FAILED - file locked
    rmdir /S /Q "{new}"
    del "%~f0"
    pause
    exit /b 1
)

echo [Updater] OK - restarting...
start "" "{app}\{exe}"
rmdir /S /Q "{new}"
del "%~f0"
"#,
            new = &new_dir_str,
            app = &app_dir_str,
            exe = &exe_name,
        )
    } else {
        format!(
            r#"@echo off
echo Updating switch-node...
timeout /t 2 /nobreak >nul
xcopy /E /Y "{new}\*" "{app}\"
if %errorlevel% neq 0 (
    echo Update failed.
    rmdir /S /Q "{new}"
    del "%~f0"
    pause
    exit /b 1
)
echo Update complete. Restarting...
start "" "{app}\{exe}"
rmdir /S /Q "{new}"
del "%~f0"
"#,
            new = &new_dir_str,
            app = &app_dir_str,
            exe = &exe_name,
        )
    };

    std::fs::write(&script_path, &script)
        .map_err(|e| format!("Failed to create update script: {}", e))?;

    log::info!(
        "[UPDATER] Spawning updater script: {}",
        script_path.display()
    );

    std::process::Command::new("cmd")
        .args(["/C", script_path.to_str().unwrap_or("updater.bat")])
        .spawn()
        .map_err(|e| format!("Failed to start update script: {}", e))?;

    Ok(())
}
