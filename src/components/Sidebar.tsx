import { useNodeStore } from "../stores/useNodeStore";
import { t } from "../lib/i18n";
import { Search, RefreshCw } from "lucide-react";
import { VersionList } from "./VersionList";

export function Sidebar() {
  const {
    localVersions,
    remoteVersions,
    systemNode,
    selectedVersion,
    searchQuery,
    showCurrent,
    showLts,
    showEol,
    isOffline,
    remoteLoading,
    remoteError,
    selectVersion,
    setSearchQuery,
    setShowCurrent,
    setShowLts,
    setShowEol,
    refreshRemote,
  } = useNodeStore();

  // Filter remote versions
  const localSet = new Set(localVersions.map((v) => v.version));
  const remoteMap = new Map(remoteVersions.map((v) => [v.version, v]));
  const filteredRemote = remoteVersions.filter((v) => {
    if (localSet.has(v.version)) return false;
    if (v.status === "current" && !showCurrent) return false;
    if ((v.status === "activeLts" || v.status === "maintenanceLts") && !showLts) return false;
    if (v.status === "eol" && !showEol) return false;
    if (searchQuery && !v.version.includes(searchQuery)) return false;
    return true;
  });

  return (
    <div className="flex flex-col h-full bg-gray-50 dark:bg-gray-800/50">
      {/* Search */}
      <div className="p-3 border-b border-gray-200 dark:border-gray-700">
        <div className="relative">
          <Search
            size={14}
            className="absolute left-2.5 top-1/2 -translate-y-1/2 text-gray-400"
          />
          <input
            type="text"
            placeholder={t("sidebar.search")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-8 pr-2 py-1.5 text-sm bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-1 focus:ring-node-green text-gray-800 dark:text-gray-200 placeholder-gray-400"
          />
        </div>
      </div>

      {/* Filters */}
      <div className="px-3 py-2 border-b border-gray-200 dark:border-gray-700 flex flex-col gap-1.5">
        <label className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400 cursor-pointer">
          <input
            type="checkbox"
            checked={showCurrent}
            onChange={(e) => setShowCurrent(e.target.checked)}
            className="rounded border-gray-300 text-node-green focus:ring-node-green w-3.5 h-3.5"
          />
          {t("sidebar.showCurrent")}
        </label>
        <label className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400 cursor-pointer">
          <input
            type="checkbox"
            checked={showLts}
            onChange={(e) => setShowLts(e.target.checked)}
            className="rounded border-gray-300 text-node-green focus:ring-node-green w-3.5 h-3.5"
          />
          {t("sidebar.showLts")}
        </label>
        <label className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400 cursor-pointer">
          <input
            type="checkbox"
            checked={showEol}
            onChange={(e) => setShowEol(e.target.checked)}
            className="rounded border-gray-300 text-node-green focus:ring-node-green w-3.5 h-3.5"
          />
          {t("sidebar.showEol")}
        </label>
      </div>

      {/* Installed Versions */}
      <div className="flex-1 overflow-y-auto">
        <div className="px-3 py-2">
          <h3 className="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1">
            {t("sidebar.installed", { count: localVersions.length })}
          </h3>
          {localVersions.length === 0 ? (
            <p className="text-xs text-gray-400 dark:text-gray-500 py-2">
              {t("sidebar.noInstalled")}
            </p>
          ) : (
            <VersionList
              versions={localVersions.map((v) => {
                const remote = remoteMap.get(v.version);
                return {
                  version: v.version,
                  status: "installed" as const,
                  isActive: v.isActive,
                  isCorrupted: v.isCorrupted,
                  lts: remote?.lts ?? null,
                  statusColor: remote?.statusColor ?? "current",
                  statusLabel: v.isCorrupted
                    ? t("version.corrupted")
                    : remote?.statusLabel ?? "",
                };
              })}
              selectedVersion={selectedVersion}
              onSelect={selectVersion}
            />
          )}

          {/* System Node.js (detected via PATH, not managed) */}
          {systemNode && !systemNode.inManagedDir && (
            <div className="mt-3 border-t border-dashed border-yellow-300 dark:border-yellow-700 pt-2">
              <p className="text-[10px] text-yellow-600 dark:text-yellow-400 mb-1">
                ⚡ 系统检测到 Node.js（非管理目录）
              </p>
              <p className="text-xs text-gray-500 dark:text-gray-400 font-mono truncate">
                {systemNode.version} — {systemNode.path}
              </p>
              <p className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5">
                请在设置中配置 nodeRoot 或将其移动到管理目录
              </p>
            </div>
          )}
        </div>

        {/* Available Versions */}
        <div className="px-3 py-2 border-t border-gray-200 dark:border-gray-700">
          <h3 className="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1">
            {t("sidebar.available", { count: filteredRemote.length })}
            {isOffline && (
              <span className="ml-1 text-yellow-500 text-[10px]">
                {t("sidebar.offline")}
              </span>
            )}
            {remoteLoading && (
              <span className="ml-1 inline-block w-3 h-3 border-2 border-gray-300 border-t-node-green rounded-full animate-spin" />
            )}
          </h3>

          {/* Remote error with retry */}
          {remoteError && !remoteLoading && filteredRemote.length === 0 && (
            <div className="py-2">
              <p className="text-xs text-red-500 dark:text-red-400 mb-1.5">
                {remoteError}
              </p>
              <button
                onClick={() => refreshRemote(true)}
                className="flex items-center gap-1 text-xs text-node-green hover:text-node-green/80 font-medium"
              >
                <RefreshCw size={11} />
                点击重试
              </button>
            </div>
          )}

          {!remoteError && filteredRemote.length === 0 && !remoteLoading ? (
            <p className="text-xs text-gray-400 dark:text-gray-500 py-2">
              {t("sidebar.noAvailable")}
            </p>
          ) : (
            <VersionList
              versions={filteredRemote.map((v) => ({
                version: v.version,
                status: "available" as const,
                isActive: false,
                isCorrupted: false,
                lts: v.lts,
                statusColor: v.statusColor,
                statusLabel: v.statusLabel,
              }))}
              selectedVersion={selectedVersion}
              onSelect={selectVersion}
            />
          )}
        </div>
      </div>
    </div>
  );
}
