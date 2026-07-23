use crate::config::AppConfig;
use crate::error::AppError;
use log::{error, info};
use serde::Serialize;
use std::fs;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalVersion {
    pub version: String,
    pub path: String,
    pub is_active: bool,
    pub is_corrupted: bool,
}

/// Scan the node_root directory for installed versions
pub fn scan_local_versions(config: &AppConfig) -> Vec<LocalVersion> {
    let node_root = Path::new(&config.node_root);
    if !node_root.exists() || !node_root.is_dir() {
        return Vec::new();
    }

    let active_version = get_active_version(config);
    let mut versions = Vec::new();

    if let Ok(entries) = fs::read_dir(node_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    // Check if directory name matches X.Y.Z pattern
                    if is_version_pattern(dir_name) {
                        let node_exe = path.join("node.exe");
                        let is_corrupted = !node_exe.exists();
                        let is_active = active_version
                            .as_ref()
                            .map(|v| v == dir_name)
                            .unwrap_or(false);

                        versions.push(LocalVersion {
                            version: dir_name.to_string(),
                            path: path.to_string_lossy().to_string(),
                            is_active,
                            is_corrupted,
                        });
                    }
                }
            }
        }
    }

    // Sort by version (newest first)
    versions.sort_by(|a, b| {
        compare_versions(&b.version, &a.version)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    versions
}

/// Get the currently active version from the symlink
pub fn get_active_version(config: &AppConfig) -> Option<String> {
    let node_root = Path::new(&config.node_root);
    let symlink_path = node_root.join(&config.symlink_name);

    if !symlink_path.exists() {
        return None;
    }

    // Try to read the symlink target
    if let Ok(target) = fs::read_link(&symlink_path) {
        if let Some(dir_name) = target.file_name().and_then(|n| n.to_str()) {
            return Some(dir_name.to_string());
        }
    }

    // On Windows, symlinks may not be readable by fs::read_link for non-admin
    // Try to resolve the path and extract version
    if let Ok(canonical) = symlink_path.canonicalize() {
        if let Some(dir_name) = canonical.file_name().and_then(|n| n.to_str()) {
            if is_version_pattern(dir_name) {
                return Some(dir_name.to_string());
            }
        }
    }

    None
}

/// Check if a string matches X.Y.Z version pattern
fn is_version_pattern(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}

/// Compare two version strings
fn compare_versions(a: &str, b: &str) -> Result<std::cmp::Ordering, AppError> {
    let parse = |v: &str| -> Result<(u32, u32, u32), AppError> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() != 3 {
            return Err(AppError::Parse(format!("Invalid version: {}", v)));
        }
        Ok((
            parts[0]
                .parse()
                .map_err(|_| AppError::Parse(format!("Invalid version: {}", v)))?,
            parts[1]
                .parse()
                .map_err(|_| AppError::Parse(format!("Invalid version: {}", v)))?,
            parts[2]
                .parse()
                .map_err(|_| AppError::Parse(format!("Invalid version: {}", v)))?,
        ))
    };

    let a_parts = parse(a)?;
    let b_parts = parse(b)?;
    Ok(a_parts.cmp(&b_parts))
}

/// Switch to a specific version.
///
/// Uses a directory junction (`mklink /J`) which does NOT require admin privileges.
/// PATH is NOT modified during switch — the `current` junction target changes,
/// but the PATH entry (`{nodeRoot}\current`) stays the same.
pub fn switch_version(config: &AppConfig, version: &str) -> Result<(), AppError> {
    let node_root = Path::new(&config.node_root);
    let target_dir = node_root.join(version);
    let node_exe = target_dir.join("node.exe");

    // 1. Verify target exists
    if !node_exe.exists() {
        return Err(AppError::VersionNotFound(format!(
            "Version {} is not installed at {:?}",
            version, target_dir
        )));
    }

    // 2. Check if already active
    if let Some(active) = get_active_version(config) {
        if active == version {
            info!("Version {} is already active, no switch needed", version);
            return Ok(());
        }
    }

    // 3. Remove old junction + create new one (mklink /J, no admin needed)
    perform_switch(config, version)
}

/// Check if current process has administrator privileges (Windows)
pub fn is_admin() -> bool {
    #[cfg(windows)]
    {
        // Use CheckTokenMembership via winapi-like approach
        // For simplicity, try to create a test symlink and check if it succeeds
        // A proper implementation would use OpenProcessToken + CheckTokenMembership
        let test_path = std::env::temp_dir().join("switch-node-admin-test");
        let test_target = std::env::temp_dir().join("switch-node-admin-test-target");

        // Try creating a directory symlink — only admins can do this on Windows
        let result = std::process::Command::new("cmd")
            .args([
                "/c",
                "mklink",
                "/D",
                &test_path.to_string_lossy(),
                &test_target.to_string_lossy(),
            ])
            .output();

        // Cleanup
        let _ = fs::remove_dir(&test_path);
        let _ = fs::remove_dir(&test_target);

        match result {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(not(windows))]
    {
        // On Unix, check euid
        unsafe { libc::geteuid() == 0 }
    }
}

/// Perform junction-based switch.
///
/// Uses `cmd /c rmdir` + `mklink /J` to replace the junction atomically.
/// Neither operation requires admin privileges on Windows.
/// PATH is NOT touched — the caller (setup wizard) should have already
/// ensured `{nodeRoot}\current` is in the system PATH once during init.
fn perform_switch(config: &AppConfig, version: &str) -> Result<(), AppError> {
    let node_root = Path::new(&config.node_root);
    let target_dir = node_root.join(version);
    let link_path = node_root.join(&config.symlink_name);

    info!(
        "Switching to version {} (junction: {:?} -> {:?})",
        version, link_path, target_dir
    );

    // Remove existing junction / symlink / empty directory.
    // Uses `cmd /c rmdir` which is safe for junctions:
    //   - junction → removes the junction ONLY (target untouched)
    //   - empty real dir → removes it
    //   - non-empty real dir → fails (prevents accidental data loss)
    if link_path.exists() {
        let output = Command::new("cmd")
            .args(["/c", "rmdir", &link_path.to_string_lossy()])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If rmdir failed because it's a non-empty real directory, bail out
            if link_path.exists() {
                error!("Failed to remove existing link at {:?}: {}", link_path, stderr);
                return Err(AppError::Process(format!(
                    "Failed to remove existing link: {}",
                    stderr.trim()
                )));
            }
        }
    }

    // Create new directory junction using mklink /J (no admin required)
    let output = Command::new("cmd")
        .args([
            "/c",
            "mklink",
            "/J",
            &link_path.to_string_lossy(),
            &target_dir.to_string_lossy(),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("mklink /J failed: {}", stderr);
        return Err(AppError::Process(format!("Failed to create junction: {}", stderr)));
    }

    // Broadcast WM_SETTINGCHANGE so already-open terminals pick up the change
    #[cfg(windows)]
    {
        broadcast_environment_change();
    }

    info!("Successfully switched to version {}", version);
    Ok(())
}

/// Broadcast WM_SETTINGCHANGE to notify system of environment changes
#[cfg(windows)]
fn broadcast_environment_change() {
    use std::os::windows::ffi::OsStrExt;
    unsafe {

        let env = "Environment";
        let env_wide: Vec<u16> = std::ffi::OsStr::new(env)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, 0, (LPARAM)L"Environment", ...)
        extern "system" {
            fn SendMessageTimeoutW(
                hWnd: isize,
                Msg: u32,
                wParam: usize,
                lParam: *const u16,
                fuFlags: u32,
                uTimeout: u32,
                lpdwResult: *mut usize,
            ) -> isize;
        }

        let mut result: usize = 0;
        SendMessageTimeoutW(
            -1isize,          // HWND_BROADCAST = 0xFFFF = -1
            0x001A,           // WM_SETTINGCHANGE
            0,
            env_wide.as_ptr(),
            0x0002,           // SMTO_ABORTIFHUNG
            5000,
            &mut result,
        );
    }
}

#[cfg(not(windows))]
fn broadcast_environment_change() {
    // no-op on non-Windows
}

/// Update the Machine-level PATH environment variable.
///
/// This is a ONE-TIME setup operation called during the initialization wizard.
/// After this, day-to-day version switches via [`perform_switch`] only update
/// the junction target — PATH is never touched again.
pub fn update_system_path(node_root: &Path, symlink_name: &str) -> Result<(), AppError> {
    // Use PowerShell to update Machine PATH
    let current_path = node_root.join(symlink_name);
    let ps_script = format!(
        r#"
$nodeRoot = "{}"
$currentPath = "{}"
$symlinkName = "{}"

try {{
    # Read Machine PATH
    $machinePath = [Environment]::GetEnvironmentVariable("PATH", "Machine")
    if ($null -eq $machinePath) {{ $machinePath = "" }}

    # Split into entries
    $paths = $machinePath -split ';' | Where-Object {{ $_ -ne "" }}

    # Remove any existing version-specific paths
    $paths = $paths | Where-Object {{
        $_ -notmatch [regex]::Escape($nodeRoot) + "\\\d+\.\d+\.\d+$"
    }}

    # Ensure current symlink path is present
    if ($currentPath -notin $paths) {{
        $paths = @($currentPath) + $paths
    }}

    # Save back
    $newPath = $paths -join ';'
    [Environment]::SetEnvironmentVariable("PATH", $newPath, "Machine")
    Write-Output "OK"
}} catch {{
    Write-Error $_.Exception.Message
    exit 1
}}
"#,
        node_root.display(),
        current_path.display(),
        symlink_name
    );

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &ps_script,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to update system PATH: {}", stderr);
        return Err(AppError::Process(format!(
            "Failed to update system PATH: {}",
            stderr
        )));
    }

    Ok(())
}

/// Remove a version directory
pub fn remove_version(config: &AppConfig, version: &str) -> Result<(), AppError> {
    let node_root = Path::new(&config.node_root);
    let version_dir = node_root.join(version);

    // Don't remove the active version
    if let Some(active) = get_active_version(config) {
        if active == version {
            return Err(AppError::Generic(
                "Cannot remove the currently active version. Please switch to another version first."
                    .to_string(),
            ));
        }
    }

    if !version_dir.exists() {
        return Err(AppError::VersionNotFound(format!(
            "Version {} is not installed",
            version
        )));
    }

    // Check for file locks
    let node_exe = version_dir.join("node.exe");
    if node_exe.exists() {
        // Try to check if node.exe is in use
        let check = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "(Get-Process -Name node -ErrorAction SilentlyContinue | Where-Object {{ $_.Path -eq '{}' }}).Count",
                    node_exe.display()
                ),
            ])
            .output();

        if let Ok(output) = check {
            let count: String = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if count != "0" && !count.is_empty() {
                return Err(AppError::FileInUse(
                    "node.exe is currently running. Please close all Node.js processes first."
                        .to_string(),
                ));
            }
        }
    }

    fs::remove_dir_all(&version_dir)?;
    info!("Removed version {} at {:?}", version, version_dir);
    Ok(())
}

/// Detect system-installed Node.js by checking `where node`
/// Returns (version, path) if a non-managed Node.js is found on the system
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemNodeInfo {
    pub version: String,
    pub path: String,
    pub in_managed_dir: bool,
}

/// Try to find Node.js from the system PATH (as a fallback)
pub fn detect_system_node(config: &AppConfig) -> Option<SystemNodeInfo> {
    // Scan PATH in pure Rust (no console window)
    let path_var = std::env::var("PATH").ok()?;
    let node_root = config.node_root.as_str();

    #[cfg(windows)]
    let (exe_name, separator) = ("node.exe", ';');
    #[cfg(not(windows))]
    let (exe_name, separator) = ("node", ':');

    for dir in path_var.split(separator) {
        let candidate = Path::new(dir).join(exe_name);
        if !candidate.exists() {
            continue;
        }

        let path_str = candidate.to_string_lossy().to_string();

        // Check if inside the managed nodeRoot
        let in_managed = candidate
            .ancestors()
            .any(|a| a.to_string_lossy() == node_root);

        // Get version
        #[cfg(windows)]
        let version_cmd = std::process::Command::new(&path_str)
            .arg("-v")
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
        #[cfg(not(windows))]
        let version_cmd = std::process::Command::new(&path_str)
            .arg("-v")
            .output();

        if let Ok(version_output) = version_cmd {
            if version_output.status.success() {
                let version = String::from_utf8_lossy(&version_output.stdout)
                    .trim()
                    .trim_start_matches('v')
                    .to_string();
                if !version.is_empty() {
                    return Some(SystemNodeInfo {
                        version,
                        path: path_str,
                        in_managed_dir: in_managed,
                    });
                }
            }
        }
    }

    None
}
