import { useNodeStore } from "../stores/useNodeStore";
import { t } from "../lib/i18n";
import { RefreshCw } from "lucide-react";

export function StatusBar() {
  const {
    activeVersion,
    statusMessage,
    statusType,
    isOffline,
    remoteLoading,
    refreshLocal,
    refreshRemote,
  } = useNodeStore();

  const handleRefresh = async () => {
    await refreshLocal();
    await refreshRemote(true);
  };

  const statusColors: Record<string, string> = {
    ok: "bg-node-green",
    busy: "bg-yellow-500 animate-pulse",
    error: "bg-red-500",
    offline: "bg-yellow-500",
  };

  return (
    <div className="flex items-center justify-between h-8 px-3 bg-gray-100 dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700 text-xs flex-shrink-0">
      {/* Left: Status */}
      <div className="flex items-center gap-2">
        <div
          className={`w-2 h-2 rounded-full ${statusColors[statusType] || "bg-gray-400"}`}
        />
        <span className="text-gray-600 dark:text-gray-400">
          {statusMessage || (activeVersion
            ? t("status.ready", { version: activeVersion })
            : t("status.noActive"))}
        </span>
        {isOffline && (
          <span className="text-yellow-600 dark:text-yellow-400 font-medium">
            {t("status.offline")}
          </span>
        )}
      </div>

      {/* Right: Refresh */}
      <button
        onClick={handleRefresh}
        disabled={remoteLoading}
        className="flex items-center gap-1 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 transition-colors disabled:opacity-50"
        title={t("common.refresh")}
      >
        <RefreshCw
          size={12}
          className={remoteLoading ? "animate-spin" : ""}
        />
        <span>{t("common.refresh")}</span>
      </button>
    </div>
  );
}
