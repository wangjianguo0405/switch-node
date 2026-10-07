use crate::config::AppConfig;
use crate::error::AppError;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::fs;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
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

// ── PATH configuration ───────────────────────────────────────────────────────
//
// `{nodeRoot}\current` has to be on PATH for `node` to resolve in a terminal.
// It is written once, here, and never touched again — day-to-day version
// switches only re-point the junction (see `perform_switch`).

/// Machine entries precede user entries in the effective PATH, so only a
/// machine entry can outrank an already-installed Node.js.
const MACHINE_ENV_KEY: (&str, &str) = (
    "LocalMachine",
    r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
);
const USER_ENV_KEY: (&str, &str) = ("CurrentUser", "Environment");

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathStatus {
    /// `{nodeRoot}\current` is present in the machine or user PATH.
    pub configured: bool,
    /// Which PATH it was found in: `"machine"`, `"user"`, or `None`.
    pub scope: Option<String>,
    /// The directory that needs to be on PATH, for display in the UI.
    pub link_path: String,
    /// The current process is elevated, so the machine PATH is writable.
    pub is_admin: bool,
    /// A `node.exe` outside `nodeRoot` that precedes our entry and would
    /// therefore win. `None` when nothing shadows us.
    pub shadowed_by: Option<String>,
    /// Whether the call that produced this status actually changed the
    /// environment. Always `false` from [`check_path_status`], which only reads;
    /// [`configure_path`] sets it when it had to write.
    pub written: bool,
}

#[derive(Deserialize)]
struct RawPathEnvironment {
    machine: String,
    user: String,
    admin: bool,
}

fn current_link_path(config: &AppConfig) -> PathBuf {
    Path::new(&config.node_root).join(&config.symlink_name)
}

/// Windows paths are case-insensitive and tolerate a trailing separator.
fn normalize_path_entry(entry: &str) -> String {
    entry.trim().trim_end_matches('\\').to_lowercase()
}

fn split_path(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(String::from)
        .collect()
}

/// Read both PATH scopes straight out of the registry.
///
/// The process environment is deliberately not used: it is a snapshot taken at
/// launch, so it goes stale the moment we write, and it cannot say which scope
/// an entry came from.
fn read_path_environment() -> Result<RawPathEnvironment, AppError> {
    const SCRIPT: &str = r#"
$m = [Environment]::GetEnvironmentVariable("PATH", "Machine"); if ($null -eq $m) { $m = "" }
$u = [Environment]::GetEnvironmentVariable("PATH", "User"); if ($null -eq $u) { $u = "" }
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$json = [pscustomobject]@{ machine = $m; user = $u; admin = $admin } | ConvertTo-Json -Compress

# Leave as pure ASCII. On a CJK Windows the console codepage is CP936/CP932, so a
# non-ASCII path (e.g. C:\用户\...) comes back as bytes that are not valid UTF-8
# and serde_json rejects the whole response.
-join ($json.ToCharArray() | ForEach-Object {
    if ([int]$_ -gt 127) { "\u{0:x4}" -f [int]$_ } else { $_ }
})
"#;

    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        SCRIPT,
    ]);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(AppError::Process(format!(
            "Failed to read PATH: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|e| AppError::Parse(format!("Failed to parse PATH environment: {}", e)))
}

fn build_path_status(config: &AppConfig, env: &RawPathEnvironment) -> PathStatus {
    let link_path = current_link_path(config);
    let link_norm = normalize_path_entry(&link_path.to_string_lossy());
    let root_norm = normalize_path_entry(&config.node_root);

    let machine = split_path(&env.machine);
    let user = split_path(&env.user);

    let in_machine = machine.iter().any(|e| normalize_path_entry(e) == link_norm);
    let in_user = user.iter().any(|e| normalize_path_entry(e) == link_norm);
    let scope = if in_machine {
        Some("machine".to_string())
    } else if in_user {
        Some("user".to_string())
    } else {
        None
    };

    // Walk the effective PATH in order and stop at our own entry: any node.exe
    // before that point is what actually runs.
    let mut shadowed_by = None;
    let root_prefix = format!("{}\\", root_norm);
    for entry in machine.iter().chain(user.iter()) {
        let norm = normalize_path_entry(entry);
        if norm == link_norm {
            break;
        }
        if norm == root_norm || norm.starts_with(&root_prefix) {
            continue; // inside our own nodeRoot, not a foreign install
        }
        let candidate = Path::new(entry).join("node.exe");
        if candidate.exists() {
            shadowed_by = Some(candidate.to_string_lossy().to_string());
            break;
        }
    }

    PathStatus {
        configured: scope.is_some(),
        scope,
        link_path: link_path.to_string_lossy().to_string(),
        is_admin: env.admin,
        shadowed_by,
        written: false,
    }
}

/// Report whether `{nodeRoot}\current` is on PATH and whether anything would
/// shadow it. Read-only — never modifies the environment.
pub fn check_path_status(config: &AppConfig) -> Result<PathStatus, AppError> {
    let env = read_path_environment()?;
    let status = build_path_status(config, &env);
    info!(
        "PATH check: configured={} scope={:?} shadowed_by={:?}",
        status.configured, status.scope, status.shadowed_by
    );
    Ok(status)
}

/// Add `{nodeRoot}\current` to PATH, once.
///
/// Prefers the machine PATH so the entry outranks an already-installed Node.js,
/// and falls back to the user PATH (which needs no elevation) when that is
/// refused. Writes nothing if the entry is already there.
pub fn configure_path(config: &AppConfig) -> Result<PathStatus, AppError> {
    let link_path = current_link_path(config);

    let existing = check_path_status(config)?;
    if existing.configured {
        info!(
            "{} is already on PATH ({} scope), leaving it alone",
            existing.link_path,
            existing.scope.as_deref().unwrap_or("unknown")
        );
        return Ok(existing);
    }

    let machine_err = match write_path_entry(config, &link_path, MACHINE_ENV_KEY.0, MACHINE_ENV_KEY.1)
    {
        Ok(()) => return finish_path_write(config),
        Err(e) => e,
    };
    info!(
        "Machine PATH write failed ({}), falling back to the user PATH",
        machine_err
    );

    match write_path_entry(config, &link_path, USER_ENV_KEY.0, USER_ENV_KEY.1) {
        Ok(()) => finish_path_write(config),
        Err(user_err) => {
            error!("User PATH write failed too: {}", user_err);
            if matches!(machine_err, AppError::AdminRequired) {
                Err(AppError::AdminRequired)
            } else {
                Err(machine_err)
            }
        }
    }
}

/// Announce the change, then re-read so the caller sees the post-write state.
fn finish_path_write(config: &AppConfig) -> Result<PathStatus, AppError> {
    broadcast_environment_change();
    let mut status = check_path_status(config)?;
    status.written = true;
    Ok(status)
}

/// Prepend the link to PATH in the given registry scope.
///
/// Goes through the .NET registry API rather than PowerShell's registry
/// provider, for two reasons. `Get-ItemProperty` expands `%SystemRoot%`-style
/// references on read, so writing its result back would freeze every variable
/// in the user's PATH into a literal path; `DoNotExpandEnvironmentNames` keeps
/// the value as found. And a denied write is reported by exit code rather than
/// by message, since the registry error text is localized.
fn write_path_entry(
    config: &AppConfig,
    link_path: &Path,
    hive: &str,
    sub_key: &str,
) -> Result<(), AppError> {
    const SCRIPT: &str = r#"
$ErrorActionPreference = "Stop"
$link     = $env:SWITCH_NODE_LINK
$root     = $env:SWITCH_NODE_ROOT
$hiveName = $env:SWITCH_NODE_HIVE
$subKey   = $env:SWITCH_NODE_SUBKEY

try {
    $hive = if ($hiveName -eq "LocalMachine") {
        [Microsoft.Win32.Registry]::LocalMachine
    } else {
        [Microsoft.Win32.Registry]::CurrentUser
    }

    $key = $hive.OpenSubKey($subKey, $true)
    if ($null -eq $key) { $key = $hive.CreateSubKey($subKey) }

    $raw = $key.GetValue("Path", "", [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    if ($null -eq $raw) { $raw = "" }

    $entries = $raw -split ';' | Where-Object { $_.Trim() -ne "" }

    # Drop per-version directories left behind by older builds of this app
    $versionPattern = '^' + [regex]::Escape($root) + '\\\d+\.\d+\.\d+$'
    $entries = $entries | Where-Object { $_ -notmatch $versionPattern }

    # Drop any existing copy of our own entry so it does not get duplicated
    $linkTrimmed = $link.TrimEnd('\')
    $entries = $entries | Where-Object { $_.Trim().TrimEnd('\') -ine $linkTrimmed }

    # Prepend: a later entry loses to an earlier node.exe
    $entries = @($link) + $entries

    $key.SetValue("Path", ($entries -join ';'), [Microsoft.Win32.RegistryValueKind]::ExpandString)
    $key.Close()
    Write-Output "OK"
} catch [System.Security.SecurityException] {
    exit 2
} catch [System.UnauthorizedAccessException] {
    exit 2
} catch {
    Write-Error $_.Exception.Message
    exit 1
}
"#;

    let mut cmd = Command::new("powershell");
    cmd.env("SWITCH_NODE_LINK", link_path.as_os_str())
        .env("SWITCH_NODE_ROOT", &config.node_root)
        .env("SWITCH_NODE_HIVE", hive)
        .env("SWITCH_NODE_SUBKEY", sub_key)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            SCRIPT,
        ]);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let output = cmd.output()?;

    // The script exits 2 for a refused write, whatever the system language
    if output.status.code() == Some(2) {
        return Err(AppError::AdminRequired);
    }
    if !output.status.success() {
        return Err(AppError::Process(format!(
            "Failed to write {}\\{}: {}",
            hive,
            sub_key,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    info!("Added {} to {}\\{}", link_path.display(), hive, sub_key);
    Ok(())
}

/// Relaunch this executable elevated, so the machine PATH becomes writable.
///
/// The relaunch cannot happen directly: the single-instance plugin would let the
/// new copy hand its arguments to the still-running original and quit. A
/// detached helper waits for this process to exit first — the same trick
/// `updater.rs` uses for its update batch script.
pub fn relaunch_as_admin() -> Result<(), AppError> {
    const SCRIPT: &str = r#"
$p = Get-Process -Id $env:SWITCH_NODE_PID -ErrorAction SilentlyContinue
if ($p) { $p.WaitForExit() }
Start-Process -FilePath $env:SWITCH_NODE_EXE -Verb RunAs
"#;

    let exe = std::env::current_exe()?;

    let mut cmd = Command::new("powershell");
    cmd.env("SWITCH_NODE_PID", std::process::id().to_string())
        .env("SWITCH_NODE_EXE", exe.as_os_str())
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            SCRIPT,
        ]);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    cmd.spawn()?;
    info!("Elevation helper launched; waiting for this instance to exit");
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
