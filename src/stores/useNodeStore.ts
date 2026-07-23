import { create } from "zustand";
import type {
  AppConfig,
  LocalVersion,
  RemoteVersion,
  DownloadProgress,
  SystemNodeInfo,
  UpdateInfo,
  UpdateConfig,
  UpdateStatus,
} from "../lib/types";
import * as commands from "../lib/commands";
import { setLanguage } from "../lib/i18n";
import { listen } from "@tauri-apps/api/event";

interface NodeStore {
  // ── Config ──
  config: AppConfig | null;
  loading: boolean;
  error: string | null;

  // ── Versions ──
  localVersions: LocalVersion[];
  remoteVersions: RemoteVersion[];
  activeVersion: string | null;

  // ── System Node ──
  systemNode: SystemNodeInfo | null;

  // ── Remote status ──
  remoteLoading: boolean;
  remoteError: string | null;
  isOffline: boolean;

  // ── Download ──
  downloadProgress: DownloadProgress | null;

  // ── UI State ──
  selectedVersion: string | null;
  searchQuery: string;
  showCurrent: boolean;
  showLts: boolean;
  showEol: boolean;
  settingsOpen: boolean;
  wizardStep: number;

  // ── Update ──
  updateStatus: UpdateStatus;
  updateInfo: UpdateInfo | null;
  selfUpdateConfig: UpdateConfig;
  showUpdateModal: boolean;
  updateDownloading: boolean;
  updateProgress: number;

  // ── Status ──
  statusMessage: string;
  statusType: "ok" | "busy" | "error" | "offline";

  // ── Actions ──
  initialize: () => Promise<void>;
  refreshLocal: () => Promise<void>;
  refreshRemote: (force?: boolean) => Promise<void>;
  switchVersion: (version: string) => Promise<void>;
  downloadVersion: (version: string) => Promise<void>;
  removeVersion: (version: string) => Promise<void>;
  updateConfig: (partial: Partial<AppConfig>) => Promise<void>;
  selectVersion: (version: string | null) => void;
  setSearchQuery: (q: string) => void;
  setShowCurrent: (v: boolean) => void;
  setShowLts: (v: boolean) => void;
  setShowEol: (v: boolean) => void;
  setSettingsOpen: (v: boolean) => void;
  setWizardStep: (step: number) => void;
  clearDownloadProgress: () => void;

  // ── Update Actions ──
  loadUpdateConfig: () => Promise<void>;
  handleCheckUpdate: () => Promise<void>;
  handleDownloadUpdate: () => Promise<void>;
  handleSaveUpdateConfig: (config: UpdateConfig) => Promise<void>;
  setShowUpdateModal: (show: boolean) => void;
}

export const useNodeStore = create<NodeStore>((set, get) => ({
  // ── Initial State ──
  config: null,
  loading: true,
  error: null,
  localVersions: [],
  remoteVersions: [],
  activeVersion: null,
  systemNode: null,
  remoteLoading: false,
  remoteError: null,
  isOffline: false,
  downloadProgress: null,
  selectedVersion: null,
  searchQuery: "",
  showCurrent: true,
  showLts: true,
  showEol: false,
  settingsOpen: false,
  wizardStep: 0,
  // ── Update initial ──
  updateStatus: "checking",
  updateInfo: null,
  selfUpdateConfig: {
    github_owner: "",
    github_repo: "",
    github_token: null,
    last_check: null,
    update_interval: 60,
  },
  showUpdateModal: false,
  updateDownloading: false,
  updateProgress: 0,

  statusMessage: "",
  statusType: "ok",

  // ── Initialize ──
  initialize: async () => {
    try {
      // Load config
      const config = await commands.getConfig();
      setLanguage(config.language);
      set({ config });

      // Scan local versions
      const local = await commands.scanLocalVersions();
      const active = local.find((v) => v.isActive)?.version ?? null;
      set({ localVersions: local, activeVersion: active });

      // Detect system Node.js (outside managed dir)
      try {
        const sysNode = await commands.detectSystemNode();
        if (sysNode && !sysNode.inManagedDir) {
          set({ systemNode: sysNode });
        }
      } catch {
        // Non-critical, ignore
      }

      // Check if wizard needed (only if no local AND no system node)
      if (local.length === 0 && !get().systemNode) {
        set({ wizardStep: 1, loading: false });
      } else {
        set({ loading: false });
      }

      // Update status
      if (active) {
        set({
          statusMessage: `node ${active} 已就绪`,
          statusType: "ok",
        });
      } else if (local.length > 0) {
        set({
          statusMessage: "未检测到活动版本 — 请在左侧列表中选择一个版本并点击切换",
          statusType: "error",
        });
      } else if (get().systemNode) {
        set({
          statusMessage: "检测到系统 Node.js — 请在设置中配置 nodeRoot 以管理版本",
          statusType: "ok",
        });
      }

      // Fetch remote versions in background
      get().refreshRemote(false);

      // Listen for download progress events
      listen<DownloadProgress>("download-progress", (event) => {
        set({ downloadProgress: event.payload });
      });

      // Listen for download complete
      listen<{ version: string }>("download-complete", () => {
        get().refreshLocal();
      });

      // Listen for version switched
      listen<{ version: string }>(
        "version-switched",
        () => {
          get().refreshLocal();
        }
      );

      // Listen for switch errors
      listen<{ error: string }>("switch-error", (event) => {
        set({
          statusType: "error",
          statusMessage: `${event.payload.error}`,
        });
      });
    } catch (err) {
      set({ loading: false, error: String(err) });
    }
  },

  // ── Refresh Local ──
  refreshLocal: async () => {
    try {
      const local = await commands.scanLocalVersions();
      const active = local.find((v) => v.isActive)?.version ?? null;
      set({ localVersions: local, activeVersion: active });

      if (active) {
        set({
          statusMessage: `node ${active} 已就绪`,
          statusType: "ok",
        });
      }
    } catch (err) {
      console.error("Failed to scan local versions:", err);
    }
  },

  // ── Refresh Remote ──
  refreshRemote: async (force = false) => {
    set({ remoteLoading: true, remoteError: null });
    try {
      const remote = await commands.fetchRemoteVersions(force);
      set({
        remoteVersions: remote,
        isOffline: false,
        remoteLoading: false,
        remoteError: null,
      });
    } catch (err) {
      const msg = String(err);
      if (msg.includes("OFFLINE") || msg.includes("offline") || msg.includes("Network")) {
        set({
          isOffline: true,
          remoteLoading: false,
          remoteError: "无法连接远程服务器，请检查网络或切换镜像源",
        });
      } else {
        set({
          remoteLoading: false,
          remoteError: `获取远程版本失败: ${msg}`,
        });
        console.error("Failed to fetch remote versions:", err);
      }
    }
  },

  // ── Switch Version ──
  switchVersion: async (version: string) => {
    set({ statusType: "busy", statusMessage: `正在切换到 ${version}...` });
    try {
      await commands.switchVersion(version);
      await get().refreshLocal();
    } catch (err) {
      set({
        statusType: "error",
        statusMessage: `切换失败: ${err}`,
      });
      throw err;
    }
  },

  // ── Download Version ──
  downloadVersion: async (version: string) => {
    set({
      downloadProgress: {
        stage: "connecting",
        downloaded: 0,
        total: 0,
        speedBps: 0,
        elapsedSecs: 0,
        version,
      },
      statusType: "busy",
      statusMessage: `正在下载 ${version}...`,
    });
    try {
      await commands.downloadVersion(version);
      set({ statusType: "ok", statusMessage: `${version} 安装完成` });
    } catch (err) {
      set({
        downloadProgress: {
          stage: "error",
          downloaded: 0,
          total: 0,
          speedBps: 0,
          elapsedSecs: 0,
          version,
        },
        statusType: "error",
        statusMessage: `下载失败: ${err}`,
      });
    }
  },

  // ── Remove Version ──
  removeVersion: async (version: string) => {
    try {
      await commands.removeVersion(version);
      await get().refreshLocal();
      set({ selectedVersion: null });
    } catch (err) {
      console.error("Failed to remove version:", err);
      throw err;
    }
  },

  // ── Update Config ──
  updateConfig: async (partial: Partial<AppConfig>) => {
    try {
      const newConfig = await commands.setConfig(partial);
      setLanguage(newConfig.language);
      set({ config: newConfig });
    } catch (err) {
      console.error("Failed to update config:", err);
      throw err;
    }
  },

  // ── UI Actions ──
  selectVersion: (version) => set({ selectedVersion: version }),
  setSearchQuery: (q) => set({ searchQuery: q }),
  setShowCurrent: (v) => set({ showCurrent: v }),
  setShowLts: (v) => set({ showLts: v }),
  setShowEol: (v) => set({ showEol: v }),
  setSettingsOpen: (v) => set({ settingsOpen: v }),
  setWizardStep: (step) => set({ wizardStep: step }),
  clearDownloadProgress: () => set({ downloadProgress: null }),

  // ── Update Actions ──
  loadUpdateConfig: async () => {
    try {
      const config = await commands.getUpdateConfig();
      set({ selfUpdateConfig: config });
    } catch {
      // Keep defaults
    }
  },

  handleCheckUpdate: async () => {
    set({ updateStatus: "checking" });
    try {
      const config = await commands.getUpdateConfig();
      set({ selfUpdateConfig: config });

      if (!config.github_owner || !config.github_repo) {
        set({ updateStatus: "idle" });
        return;
      }

      const info = await commands.checkUpdate();
      set({
        updateInfo: info,
        updateStatus: info.has_update ? "available" : "latest",
      });
    } catch {
      set({ updateStatus: "error" });
    }
  },

  handleDownloadUpdate: async () => {
    const { updateDownloading } = get();
    if (updateDownloading) return;
    set({ updateDownloading: true, updateProgress: 0 });

    try {
      set({ updateStatus: "checking" });
      const latest = await commands.checkUpdate();
      set({ updateInfo: latest });

      if (!latest.download_url) {
        throw new Error("Download URL not found");
      }

      // Simulated progress (app exits on completion)
      const timer = setInterval(() => {
        set((s) => ({
          updateProgress: Math.min(s.updateProgress + 10, 90),
        }));
      }, 300);

      await commands.downloadAndInstall(latest.download_url, latest.asset_name);
      clearInterval(timer);
    } catch (e) {
      set({
        updateDownloading: false,
        updateProgress: 0,
        updateStatus: "error",
      });
      console.error("Update failed:", e);
    }
  },

  handleSaveUpdateConfig: async (config: UpdateConfig) => {
    try {
      await commands.saveUpdateConfig(config);
      set({ selfUpdateConfig: config });
    } catch (e) {
      console.error("Failed to save update config:", e);
      throw e;
    }
  },

  setShowUpdateModal: (show) => set({ showUpdateModal: show }),
}));
