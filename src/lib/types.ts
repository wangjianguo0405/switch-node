// ── Configuration ──
export interface AppConfig {
  nodeRoot: string;
  symlinkName: string;
  mirror: string;
  mirrorName: "official" | "taobao" | "custom";
  architecture: "x64" | "arm64" | "x86";
  keepDownloads: boolean;
  downloadDir: string;
  minVersion: string;
  showEOL: boolean;
  prerelease: boolean;
  autoRefreshEnv: boolean;
  checkUpdates: boolean;
  updateIntervalHours: number;
  npmMirror: string;
  language: "zh-CN" | "en" | "ja";
  theme: "light" | "dark" | "system";
}

// ── Version Status ──
export type VersionStatus =
  | "current"
  | "activeLts"
  | "maintenanceLts"
  | "eol";

// ── Local Version ──
export interface LocalVersion {
  version: string;
  path: string;
  isActive: boolean;
  isCorrupted: boolean;
}

// ── Remote Version (classified) ──
export interface RemoteVersion {
  version: string;
  date: string;
  lts: string | null;
  npm: string;
  v8: string;
  openssl: string;
  files: string[];
  security: boolean;
  status: VersionStatus;
  statusLabel: string;
  statusColor: string;
  statusDescription: string;
  isLatest: boolean;
  isLatestLts: boolean;
  downloadSizeMb: number;
}

// ── Download Progress Events ──
export type DownloadStage =
  | "connecting"
  | "downloading"
  | "verifying"
  | "extracting"
  | "installing"
  | "complete"
  | "error";

export interface DownloadProgress {
  stage: DownloadStage;
  downloaded: number;
  total: number;
  speedBps: number;
  elapsedSecs: number;
  version: string;
}

// ── App Info ──
export interface AppInfo {
  version: string;
  isAdmin: boolean;
}

// ── System Node.js (detected via PATH) ──
export interface SystemNodeInfo {
  version: string;
  path: string;
  inManagedDir: boolean;
}

// ── UI State ──
// ── Update ──
export interface UpdateInfo {
  has_update: boolean;
  current_version: string;
  latest_version: string;
  release_notes: string;
  download_url: string;
  asset_name: string;
  asset_size: number;
}

export interface UpdateConfig {
  github_owner: string;
  github_repo: string;
  github_token?: string | null;
  last_check?: string | null;
  update_interval: number; // minutes, 0 = startup only
}

export type UpdateStatus = "idle" | "checking" | "latest" | "available" | "error";

export interface DownloadState {
  version: string;
  stage: DownloadStage;
  downloaded: number;
  total: number;
  speedBps: number;
  elapsedSecs: number;
  error: string | null;
}
