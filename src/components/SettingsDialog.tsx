import { useState } from "react";
import { useNodeStore } from "../stores/useNodeStore";
import { t } from "../lib/i18n";
import { X, FolderOpen, RefreshCw, ExternalLink, Check, AlertCircle, ArrowDown } from "lucide-react";
import type { AppConfig } from "../lib/types";

const TABS = [
  "settings.paths",
  "settings.download",
  "settings.versions",
  "settings.behavior",
  "settings.npm",
  "settings.ui",
  "settings.update",
  "settings.about",
];

export function SettingsDialog() {
  const { config, appVersion, setSettingsOpen, updateConfig, updateInfo, updateStatus, handleCheckUpdate, setShowUpdateModal } = useNodeStore();
  const [activeTab, setActiveTab] = useState("settings.paths");
  // Work on a local copy so edits don't mutate store directly
  const [form, setForm] = useState<AppConfig | null>(config ? { ...config } : null);

  if (!form) return null;

  const handleChange = (key: keyof AppConfig, value: unknown) => {
    setForm((prev) => (prev ? { ...prev, [key]: value } : prev));
  };

  const isAboutTab = activeTab === "settings.about";

  const handleSave = async () => {
    if (!form) return;
    try {
      await updateConfig(form);
      setSettingsOpen(false);
    } catch {
      // Error handled in store
    }
  };

  const handleBrowse = async (key: keyof AppConfig) => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        handleChange(key, selected);
      }
    } catch {
      // Fallback: do nothing, user can type manually
    }
  };

  return (
    <div className="fixed inset-0 bg-black/30 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-[650px] max-w-[90vw] h-[520px] max-h-[85vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-200">
            {t("settings.title")}
          </h2>
          <button
            onClick={() => setSettingsOpen(false)}
            className="p-1 rounded-md hover:bg-gray-100 dark:hover:bg-gray-700"
          >
            <X size={18} className="text-gray-400" />
          </button>
        </div>

        {/* Body */}
        <div className="flex flex-1 overflow-hidden">
          {/* Tabs */}
          <div className="w-36 border-r border-gray-200 dark:border-gray-700 py-2 flex-shrink-0">
            {TABS.map((tabKey) => (
              <button
                key={tabKey}
                onClick={() => setActiveTab(tabKey)}
                className={`w-full text-left px-4 py-2 text-sm transition-colors ${
                  activeTab === tabKey
                    ? "bg-node-green/10 text-node-green font-medium border-r-2 border-node-green"
                    : "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700/50"
                }`}
              >
                {t(tabKey)}
              </button>
            ))}
          </div>

          {/* Content */}
          <div className="flex-1 p-6 overflow-y-auto" key={activeTab}>
            <TabContent
              tab={activeTab}
              form={form}
              appVersion={appVersion}
              updateInfo={updateInfo}
              updateStatus={updateStatus}
              onBrowse={handleBrowse}
              onChange={handleChange}
              onCheckUpdate={handleCheckUpdate}
              onOpenUpdateModal={() => { setSettingsOpen(false); setShowUpdateModal(true); }}
            />
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 px-6 py-4 border-t border-gray-200 dark:border-gray-700">
          <button
            onClick={() => setSettingsOpen(false)}
            className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md transition-colors"
          >
            {isAboutTab ? t("common.close") : t("settings.cancel")}
          </button>
          {!isAboutTab && (
            <button
              onClick={handleSave}
              className="px-4 py-2 text-sm bg-node-green hover:bg-node-green/90 text-white rounded-md font-medium transition-colors"
            >
              {t("settings.save")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

// ── Helper form field components ──

function Field({
  label,
  value,
  onChange,
  type = "text",
  placeholder,
  action,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  placeholder?: string;
  action?: React.ReactNode;
}) {
  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
        {label}
      </label>
      <div className="flex gap-2">
        <input
          type={type}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className="flex-1 px-3 py-2 text-sm bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-1 focus:ring-node-green text-gray-800 dark:text-gray-200"
        />
        {action}
      </div>
    </div>
  );
}

function SelectField({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
        {label}
      </label>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full px-3 py-2 text-sm bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-1 focus:ring-node-green text-gray-800 dark:text-gray-200"
      >
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}

function CheckboxField({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex items-center gap-2 cursor-pointer">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="rounded border-gray-300 text-node-green focus:ring-node-green w-4 h-4"
      />
      <span className="text-sm text-gray-700 dark:text-gray-300">{label}</span>
    </label>
  );
}

// ── Tab Content (only active tab rendered) ──────────────────────

function TabContent({
  tab,
  form,
  appVersion,
  updateInfo,
  updateStatus,
  onBrowse,
  onChange,
  onCheckUpdate,
  onOpenUpdateModal,
}: {
  tab: string;
  form: AppConfig;
  appVersion: string;
  updateInfo: import("../lib/types").UpdateInfo | null;
  updateStatus: import("../lib/types").UpdateStatus;
  onBrowse: (key: keyof AppConfig) => void;
  onChange: (key: keyof AppConfig, value: unknown) => void;
  onCheckUpdate: () => void;
  onOpenUpdateModal: () => void;
}) {
  switch (tab) {
    case "settings.paths":
      return (
        <div className="space-y-4">
          <Field label={t("settings.nodeRoot")} value={form.nodeRoot} onChange={(v) => onChange("nodeRoot", v)} action={
            <button onClick={() => onBrowse("nodeRoot")} className="flex items-center gap-1 px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700 rounded hover:bg-gray-200 dark:hover:bg-gray-600">
              <FolderOpen size={12} />{t("settings.browse")}
            </button>
          } />
          <Field label={t("settings.symlinkName")} value={form.symlinkName} onChange={(v) => onChange("symlinkName", v)} />
        </div>
      );

    case "settings.download":
      return (
        <div className="space-y-4">
          <Field label={t("settings.mirror")} value={form.mirror} onChange={(v) => onChange("mirror", v)} />
          <SelectField label={t("settings.mirrorName")} value={form.mirrorName} onChange={(v) => { onChange("mirrorName", v); if (v === "official") onChange("mirror", "https://nodejs.org/dist"); if (v === "taobao") onChange("mirror", "https://npmmirror.com/mirrors/node"); }}
            options={[{ value: "official", label: t("settings.official") }, { value: "taobao", label: t("settings.taobao") }, { value: "custom", label: t("settings.custom") }]} />
          <SelectField label={t("settings.architecture")} value={form.architecture} onChange={(v) => onChange("architecture", v)}
            options={[{ value: "x64", label: "x64" }, { value: "arm64", label: "arm64" }, { value: "x86", label: "x86" }]} />
          <CheckboxField label={t("settings.keepDownloads")} checked={form.keepDownloads} onChange={(v) => onChange("keepDownloads", v)} />
          <Field label={t("settings.downloadDir")} value={form.downloadDir} onChange={(v) => onChange("downloadDir", v)} placeholder={t("settings.downloadDirPlaceholder")} action={
            <button onClick={() => onBrowse("downloadDir")} className="flex items-center gap-1 px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700 rounded hover:bg-gray-200 dark:hover:bg-gray-600">
              <FolderOpen size={12} />{t("settings.browse")}
            </button>
          } />
        </div>
      );

    case "settings.versions":
      return (
        <div className="space-y-4">
          <Field label={t("settings.minVersion")} value={form.minVersion} onChange={(v) => onChange("minVersion", v)} />
          <CheckboxField label={t("settings.showEOL")} checked={form.showEOL} onChange={(v) => onChange("showEOL", v)} />
          <CheckboxField label={t("settings.prerelease")} checked={form.prerelease} onChange={(v) => onChange("prerelease", v)} />
        </div>
      );

    case "settings.behavior":
      return (
        <div className="space-y-4">
          <CheckboxField label={t("settings.autoRefreshEnv")} checked={form.autoRefreshEnv} onChange={(v) => onChange("autoRefreshEnv", v)} />
          <CheckboxField label={t("settings.checkUpdates")} checked={form.checkUpdates} onChange={(v) => onChange("checkUpdates", v)} />
          <Field label={t("settings.updateIntervalHours")} value={String(form.updateIntervalHours)} onChange={(v) => onChange("updateIntervalHours", parseInt(v) || 1)} type="number" />
        </div>
      );

    case "settings.npm":
      return (
        <div className="space-y-4">
          <Field label={t("settings.npmMirror")} value={form.npmMirror} onChange={(v) => onChange("npmMirror", v)} />
        </div>
      );

    case "settings.ui":
      return (
        <div className="space-y-4">
          <SelectField label={t("settings.language")} value={form.language} onChange={(v) => onChange("language", v)}
            options={[{ value: "zh-CN", label: "简体中文" }, { value: "en", label: "English" }, { value: "ja", label: "日本語" }]} />
          <SelectField label={t("settings.theme")} value={form.theme} onChange={(v) => onChange("theme", v)}
            options={[{ value: "system", label: "System" }, { value: "light", label: "Light" }, { value: "dark", label: "Dark" }]} />
        </div>
      );

    case "settings.update":
      return (
        <div className="space-y-4">
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-2">{t("update.description")}</p>
          <Field label={t("update.githubOwner")} value={form.githubOwner} onChange={(v) => onChange("githubOwner", v)} placeholder="e.g. your-username" />
          <Field label={t("update.githubRepo")} value={form.githubRepo} onChange={(v) => onChange("githubRepo", v)} placeholder="e.g. switch-node" />
          <Field label={t("update.githubToken")} value={form.githubToken ?? ""} onChange={(v) => onChange("githubToken", v || null)} type="password" placeholder={t("update.githubTokenPlaceholder")} />
          <SelectField label={t("update.interval")} value={String(form.updateIntervalMinutes)} onChange={(v) => onChange("updateIntervalMinutes", parseInt(v))}
            options={[{ value: "0", label: t("update.intervalStartup") }, { value: "30", label: t("update.interval30min") }, { value: "60", label: t("update.interval1hour") }, { value: "180", label: t("update.interval3hours") }, { value: "360", label: t("update.interval6hours") }, { value: "720", label: t("update.interval12hours") }]} />
          <p className="text-xs text-gray-400 dark:text-gray-500">{t("update.intervalNote")}</p>
        </div>
      );

    case "settings.about":
      return (
        <div className="flex flex-col items-center space-y-4 py-4">
          <div className="text-center">
            <h3 className="text-xl font-bold text-gray-800 dark:text-gray-200">{t("app.title")}</h3>
            <p className="text-sm text-gray-500 dark:text-gray-400 mt-1 font-mono">v{appVersion}</p>
          </div>
          <div className="w-full max-w-xs space-y-3">
            {updateStatus === "checking" && <div className="flex items-center justify-center gap-2 text-sm text-gray-500"><RefreshCw size={14} className="animate-spin" />{t("update.checking")}</div>}
            {updateStatus === "latest" && <div className="flex items-center justify-center gap-2 text-sm text-green-600 dark:text-green-400"><Check size={14} strokeWidth={2.5} />{t("about.upToDate")}</div>}
            {updateStatus === "available" && updateInfo && (
              <div className="space-y-3">
                <div className="flex items-center justify-center gap-2 text-sm text-blue-600 dark:text-blue-400"><ArrowDown size={14} strokeWidth={2.5} />{t("about.newVersionAvailable", { version: updateInfo.latest_version })}</div>
                {updateInfo.release_notes && <div className="text-xs text-gray-600 dark:text-gray-400 bg-gray-50 dark:bg-gray-900 rounded-md p-3 max-h-32 overflow-auto whitespace-pre-wrap font-mono">{updateInfo.release_notes}</div>}
                <button onClick={onOpenUpdateModal} className="w-full py-2 px-4 bg-green-600 hover:bg-green-700 text-white text-sm font-medium rounded-md transition-colors">{t("update.updateNow")}</button>
              </div>
            )}
            {updateStatus === "error" && <div className="flex items-center justify-center gap-2 text-sm text-red-500"><AlertCircle size={14} />{t("update.checkFailed")}</div>}
            {updateStatus === "idle" && <div className="text-center text-sm text-gray-400">{t("about.clickToCheck")}</div>}
            <button onClick={onCheckUpdate} disabled={updateStatus === "checking"} className="w-full flex items-center justify-center gap-2 py-2 px-4 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 disabled:opacity-50 text-gray-700 dark:text-gray-300 text-sm font-medium rounded-md transition-colors">
              <RefreshCw size={14} className={updateStatus === "checking" ? "animate-spin" : ""} />{t("about.checkForUpdates")}
            </button>
          </div>
          <button onClick={() => { if (form.githubOwner && form.githubRepo) { import("@tauri-apps/plugin-shell").then(s => s.open(`https://github.com/${form.githubOwner}/${form.githubRepo}`)); } }} className="flex items-center gap-1 text-xs text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors">
            <ExternalLink size={12} />{t("about.viewOnGitHub")}
          </button>
        </div>
      );

    default:
      return null;
  }
}
