use crate::config::AppConfig;
use crate::error::AppError;
use log::{error, info, warn};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub stage: DownloadStage,
    pub downloaded: u64,
    pub total: u64,
    pub speed_bps: f64,
    pub elapsed_secs: f64,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadStage {
    Connecting,
    Downloading,
    Verifying,
    Extracting,
    Installing,
    Complete,
    Error,
}

/// Download and install a Node.js version
pub async fn download_and_install(
    config: &AppConfig,
    version: &str,
    progress_callback: impl Fn(DownloadProgress),
) -> Result<(), AppError> {
    let clean_version = version.trim_start_matches('v');
    let node_root = Path::new(&config.node_root);
    let target_dir = node_root.join(clean_version);

    // Build download URL
    let url = format!(
        "{}/v{}/node-v{}-win-{}.zip",
        config.mirror.trim_end_matches('/'),
        clean_version,
        clean_version,
        config.architecture
    );

    info!("Downloading from: {}", url);

    // Create temp file for download
    let temp_dir = if config.download_dir.is_empty() {
        std::env::temp_dir()
    } else {
        PathBuf::from(&config.download_dir)
    };
    fs::create_dir_all(&temp_dir)?;

    let zip_filename = format!("node-v{}-win-{}.zip", clean_version, config.architecture);
    let temp_zip = temp_dir.join(&zip_filename);

    let client = reqwest::Client::builder()
        .user_agent("switch-node/1.0")
        .timeout(std::time::Duration::from_secs(600)) // 10 min timeout for download
        .build()?;

    // Head request to get content length
    progress_callback(DownloadProgress {
        stage: DownloadStage::Connecting,
        downloaded: 0,
        total: 0,
        speed_bps: 0.0,
        elapsed_secs: 0.0,
        version: clean_version.to_string(),
    });

    let head_resp = client.head(&url).send().await?;
    if !head_resp.status().is_success() {
        return Err(AppError::Http(format!(
            "Version {} not found at {} (HTTP {})",
            clean_version,
            url,
            head_resp.status()
        )));
    }

    let total_size = head_resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    // Disk space check
    check_disk_space(node_root, total_size)?;

    // Download with retries
    let start = Instant::now();
    download_with_retry(&client, &url, &temp_zip, total_size, &progress_callback).await?;

    let download_elapsed = start.elapsed().as_secs_f64();
    info!(
        "Download complete: {} bytes in {:.1}s",
        total_size, download_elapsed
    );

    // SHA256 verification
    progress_callback(DownloadProgress {
        stage: DownloadStage::Verifying,
        downloaded: total_size,
        total: total_size,
        speed_bps: 0.0,
        elapsed_secs: download_elapsed,
        version: clean_version.to_string(),
    });

    verify_sha256(&client, config, clean_version, &temp_zip).await?;

    // Extract
    progress_callback(DownloadProgress {
        stage: DownloadStage::Extracting,
        downloaded: total_size,
        total: total_size,
        speed_bps: 0.0,
        elapsed_secs: start.elapsed().as_secs_f64(),
        version: clean_version.to_string(),
    });

    // Check if target already exists
    if target_dir.exists() {
        return Err(AppError::Generic(format!(
            "Version {} is already installed at {:?}. Remove it first or choose a different version.",
            clean_version, target_dir
        )));
    }

    extract_zip(&temp_zip, &target_dir)?;

    // Verify installation
    progress_callback(DownloadProgress {
        stage: DownloadStage::Installing,
        downloaded: total_size,
        total: total_size,
        speed_bps: 0.0,
        elapsed_secs: start.elapsed().as_secs_f64(),
        version: clean_version.to_string(),
    });

    verify_installation(&target_dir, clean_version)?;

    // Cleanup
    if config.keep_downloads {
        let dest = if config.download_dir.is_empty() {
            temp_dir
        } else {
            PathBuf::from(&config.download_dir)
        };
        let keep_path = dest.join(&zip_filename);
        if temp_zip != keep_path {
            fs::rename(&temp_zip, &keep_path)?;
        }
        info!("Kept download at: {:?}", keep_path);
    } else {
        let _ = fs::remove_file(&temp_zip);
        info!("Removed temp zip: {:?}", temp_zip);
    }

    progress_callback(DownloadProgress {
        stage: DownloadStage::Complete,
        downloaded: total_size,
        total: total_size,
        speed_bps: 0.0,
        elapsed_secs: start.elapsed().as_secs_f64(),
        version: clean_version.to_string(),
    });

    info!(
        "Successfully installed Node.js {} in {:.1}s",
        clean_version,
        start.elapsed().as_secs_f64()
    );

    Ok(())
}

/// Download with exponential backoff retry
async fn download_with_retry(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    total_size: u64,
    progress_callback: &impl Fn(DownloadProgress),
) -> Result<(), AppError> {
    let max_retries = 3;
    let mut last_error = None;

    for attempt in 0..max_retries {
        if attempt > 0 {
            let delay = 2u64.pow(attempt as u32);
            info!("Retry attempt {}/{} after {}s", attempt + 1, max_retries, delay);
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
        }

        match download_single(client, url, dest, total_size, progress_callback).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                warn!("Download attempt {} failed: {}", attempt + 1, e);
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| AppError::Network("Download failed after all retries".to_string())))
}

/// Single download attempt
async fn download_single(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    total_size: u64,
    progress_callback: &impl Fn(DownloadProgress),
) -> Result<(), AppError> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(AppError::Http(format!("HTTP {}", response.status())));
    }

    let mut file = tokio::fs::File::create(dest).await?;
    let mut downloaded: u64 = 0;
    let start = Instant::now();
    let mut last_progress = Instant::now();

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        // Throttle progress updates to ~200ms intervals
        let now = Instant::now();
        if now.duration_since(last_progress).as_millis() >= 200 {
            let elapsed = now.duration_since(start).as_secs_f64();
            let speed = if elapsed > 0.0 {
                downloaded as f64 / elapsed
            } else {
                0.0
            };

            progress_callback(DownloadProgress {
                stage: DownloadStage::Downloading,
                downloaded,
                total: total_size,
                speed_bps: speed,
                elapsed_secs: elapsed,
                version: String::new(),
            });

            last_progress = now;
        }
    }

    file.flush().await?;
    Ok(())
}

/// Verify SHA256 checksum of downloaded file
async fn verify_sha256(
    client: &reqwest::Client,
    config: &AppConfig,
    version: &str,
    zip_path: &Path,
) -> Result<(), AppError> {
    let shasums_url = format!(
        "{}/v{}/SHASUMS256.txt",
        config.mirror.trim_end_matches('/'),
        version
    );

    info!("Fetching SHA256 checksums from: {}", shasums_url);

    let response = client.get(&shasums_url).send().await?;
    if !response.status().is_success() {
        warn!("Could not fetch SHASUMS256.txt, skipping verification");
        return Ok(());
    }

    let body = response.text().await?;
    let zip_filename = format!("node-v{}-win-{}.zip", version, config.architecture);

    // Parse the expected hash from SHASUMS256.txt
    let expected_hash = body
        .lines()
        .find(|line| line.contains(&zip_filename))
        .and_then(|line| line.split_whitespace().next())
        .map(|h| h.to_lowercase());

    let expected_hash = match expected_hash {
        Some(h) => h,
        None => {
            warn!(
                "Could not find hash for {} in SHASUMS256.txt, skipping verification",
                zip_filename
            );
            return Ok(());
        }
    };

    // Compute actual hash
    let zip_data = fs::read(zip_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&zip_data);
    let actual_hash = hex::encode(hasher.finalize());

    if actual_hash != expected_hash {
        error!(
            "SHA256 mismatch! Expected: {}, Got: {}",
            expected_hash, actual_hash
        );
        let _ = fs::remove_file(zip_path);
        return Err(AppError::Sha256Mismatch {
            expected: expected_hash,
            actual: actual_hash,
        });
    }

    info!("SHA256 verification passed");
    Ok(())
}

/// Check available disk space
fn check_disk_space(target_dir: &Path, download_size: u64) -> Result<(), AppError> {
    // Need: zip file + extracted contents + 150MB buffer
    let needed = download_size + download_size * 3 + 150 * 1024 * 1024;

    // Use PowerShell to check free space
    let drive = target_dir
        .to_str()
        .and_then(|s| s.chars().next())
        .map(|c| format!("{}:", c))
        .unwrap_or_else(|| "C:".to_string());

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "(Get-PSDrive -Name {}).Free",
                drive.trim_end_matches(':')
            ),
        ])
        .output();

    match output {
        Ok(out) => {
            let free_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Ok(free) = free_str.parse::<u64>() {
                if free < needed {
                    return Err(AppError::DiskFull {
                        needed,
                        available: free,
                    });
                }
            }
        }
        Err(_) => {
            // Can't check, proceed anyway
            warn!("Could not check disk space, proceeding");
        }
    }

    Ok(())
}

/// Extract zip and strip top-level directory
fn extract_zip(zip_path: &Path, target_dir: &Path) -> Result<(), AppError> {
    info!(
        "Extracting {:?} to {:?}",
        zip_path, target_dir
    );

    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Generic(format!("Failed to open zip: {}", e)))?;

    // Determine the top-level directory prefix to strip
    let mut prefix: Option<String> = None;

    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| AppError::Generic(format!("Zip read error: {}", e)))?;
        let name = entry.name().to_string();

        // Find the common prefix (top-level dir inside zip)
        if let Some(slash_pos) = name.find('/') {
            let top_dir = &name[..slash_pos];
            if prefix.is_none() {
                prefix = Some(top_dir.to_string());
            }
        }
    }

    // Extract all files, stripping the prefix
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::Generic(format!("Zip read error: {}", e)))?;
        let name = entry.name().to_string();

        // Strip the top-level directory
        let relative_path = if let Some(ref p) = prefix {
            if name.starts_with(p) && name.len() > p.len() + 1 {
                &name[p.len() + 1..] // skip "prefix/"
            } else if name == *p || name == format!("{}/", p) {
                continue; // skip the top-level directory entry itself
            } else {
                &name
            }
        } else {
            &name
        };

        if relative_path.is_empty() {
            continue;
        }

        let dest_path = target_dir.join(relative_path);

        if entry.is_dir() || name.ends_with('/') {
            fs::create_dir_all(&dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut dest_file = fs::File::create(&dest_path)?;
            std::io::copy(&mut entry, &mut dest_file)?;
        }
    }

    info!("Extraction complete to {:?}", target_dir);
    Ok(())
}

/// Verify the installation by running node -v
fn verify_installation(target_dir: &Path, expected_version: &str) -> Result<(), AppError> {
    let node_exe = target_dir.join("node.exe");
    if !node_exe.exists() {
        return Err(AppError::Generic(format!(
            "Installation verification failed: node.exe not found at {:?}",
            node_exe
        )));
    }

    // Try running node -v
    let output = Command::new(&node_exe)
        .arg("-v")
        .output();

    match output {
        Ok(out) => {
            let version_output = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let expected = format!("v{}", expected_version);

            if version_output == expected {
                info!(
                    "Installation verified: {} reports version {}",
                    node_exe.display(),
                    version_output
                );
                Ok(())
            } else {
                Err(AppError::Generic(format!(
                    "Version mismatch after install: expected {}, got {}",
                    expected, version_output
                )))
            }
        }
        Err(e) => {
            // node.exe might need VC++ redistributable
            Err(AppError::Process(format!(
                "Failed to run node.exe -v: {}. You may need Visual C++ Redistributable.",
                e
            )))
        }
    }
}
