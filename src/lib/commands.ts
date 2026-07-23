import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  LocalVersion,
  RemoteVersion,
  AppInfo,
  SystemNodeInfo,
  UpdateInfo,
} from "./types";

// ── Config ──
export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export async function setConfig(
  partial: Partial<AppConfig>
): Promise<AppConfig> {
  return invoke<AppConfig>("set_config", { partial });
}

// ── Versions ──
export async function scanLocalVersions(): Promise<LocalVersion[]> {
  return invoke<LocalVersion[]>("scan_local_versions");
}

export async function fetchRemoteVersions(
  force = false
): Promise<RemoteVersion[]> {
  return invoke<RemoteVersion[]>("fetch_remote_versions", { force });
}

// ── Download ──
export async function downloadVersion(version: string): Promise<string> {
  return invoke<string>("download_version", { version });
}

// ── Switch ──
export async function switchVersion(version: string): Promise<string> {
  return invoke<string>("switch_version", { version });
}

// ── Misc ──
export async function getActiveVersion(): Promise<string | null> {
  return invoke<string | null>("get_active_version");
}

export async function removeVersion(version: string): Promise<void> {
  return invoke<void>("remove_version", { version });
}

export async function openReleaseNotes(version: string): Promise<void> {
  return invoke<void>("open_release_notes", { version });
}

export async function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}

export async function detectSystemNode(): Promise<SystemNodeInfo | null> {
  return invoke<SystemNodeInfo | null>("detect_system_node");
}

// ── Update ──
export async function checkUpdate(): Promise<UpdateInfo> {
  return invoke<UpdateInfo>("check_update");
}

export async function downloadAndInstall(
  downloadUrl: string,
  assetName: string
): Promise<void> {
  return invoke<void>("download_and_install", {
    downloadUrl,
    assetName,
  });
}
