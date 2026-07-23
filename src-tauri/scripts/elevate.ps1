param(
    [Parameter(Mandatory=$true)]
    [string]$NodeRoot,
    [Parameter(Mandatory=$true)]
    [string]$SymlinkName,
    [Parameter(Mandatory=$true)]
    [string]$TargetVersion
)

$ErrorActionPreference = "Stop"

$LinkPath = Join-Path $NodeRoot $SymlinkName
$TargetPath = Join-Path $NodeRoot $TargetVersion

Write-Host "elevate.ps1: NodeRoot=$NodeRoot, Target=$TargetVersion"

# 1. Verify target exists
$nodeExe = Join-Path $TargetPath "node.exe"
if (-not (Test-Path $nodeExe)) {
    Write-Error "Target node.exe not found: $nodeExe"
    exit 1
}

# 2. Remove old symlink/junction
if (Test-Path $LinkPath) {
    Write-Host "Removing existing link: $LinkPath"
    try {
        Remove-Item -Path $LinkPath -Force -Recurse -ErrorAction Stop
    } catch {
        Write-Error "Failed to remove existing link: $_"
        exit 1
    }
}

# 3. Create new directory symlink
Write-Host "Creating symlink: $LinkPath -> $TargetPath"
$result = cmd /c mklink /D "`"$LinkPath`"" "`"$TargetPath`"" 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Error "mklink failed: $result"
    exit 1
}

# 4. Update Machine PATH
Write-Host "Updating Machine PATH..."
$machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
if ($null -eq $machinePath) { $machinePath = "" }

$entries = $machinePath -split ';' | Where-Object { $_ -ne "" }

# Remove all fixed-version paths like D:\...\nodejs\X.Y.Z
$pattern = "^$([regex]::Escape($NodeRoot))\\\d+\.\d+\.\d+$"
$filtered = $entries | Where-Object { $_ -notmatch $pattern }

# Ensure current symlink path is present
if ($LinkPath -notin $filtered) {
    $filtered += $LinkPath
}

$newPath = $filtered -join ';'
[Environment]::SetEnvironmentVariable("Path", $newPath, "Machine")
Write-Host "Machine PATH updated."

# 5. Broadcast environment change
Write-Host "Broadcasting WM_SETTINGCHANGE..."
$HWND_BROADCAST = [IntPtr]0xFFFF
$WM_SETTINGCHANGE = 0x001A
$SMTO_ABORTIFHUNG = 0x0002

$sig = @'
[DllImport("user32.dll", CharSet = CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(
    IntPtr hWnd,
    uint Msg,
    UIntPtr wParam,
    string lParam,
    uint fuFlags,
    uint uTimeout,
    out UIntPtr lpdwResult
);
'@
$type = Add-Type -MemberDefinition $sig -Name "Win32" -Namespace "SendMessage" -PassThru
$result = [UIntPtr]::Zero
$type::SendMessageTimeout($HWND_BROADCAST, $WM_SETTINGCHANGE, [UIntPtr]::Zero, "Environment", $SMTO_ABIFHUNG, 5000, [ref] $result)

Write-Host "elevate.ps1 completed successfully."
exit 0
