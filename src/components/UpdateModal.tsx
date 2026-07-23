import { X, Download, ExternalLink } from "lucide-react";
import { useNodeStore } from "../stores/useNodeStore";
import { t } from "../lib/i18n";

export function UpdateModal() {
  const showUpdateModal = useNodeStore((s) => s.showUpdateModal);
  const setShowUpdateModal = useNodeStore((s) => s.setShowUpdateModal);
  const updateInfo = useNodeStore((s) => s.updateInfo);
  const updateStatus = useNodeStore((s) => s.updateStatus);
  const updateDownloading = useNodeStore((s) => s.updateDownloading);
  const updateProgress = useNodeStore((s) => s.updateProgress);
  const handleDownloadUpdate = useNodeStore((s) => s.handleDownloadUpdate);

  if (!showUpdateModal) return null;

  const hasUpdate = updateStatus === "available";
  const isLatest = updateStatus === "latest";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[480px] max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-base font-semibold text-gray-900 dark:text-gray-100">
            {t("update.title")}
          </h2>
          <button
            onClick={() => setShowUpdateModal(false)}
            className="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded hover:bg-gray-100 dark:hover:bg-gray-700"
          >
            <X size={16} />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-auto px-5 py-4 space-y-4">
          {/* Version comparison */}
          {updateInfo && (
            <div className="flex items-center justify-center gap-4 py-2">
              <div className="text-center">
                <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  {t("update.currentVersion")}
                </div>
                <div className="text-lg font-mono font-semibold text-gray-700 dark:text-gray-300">
                  v{updateInfo.current_version}
                </div>
              </div>
              <div className="text-2xl text-gray-400">→</div>
              <div className="text-center">
                <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  {t("update.latestVersion")}
                </div>
                <div className="text-lg font-mono font-semibold text-green-600 dark:text-green-400">
                  v{updateInfo.latest_version}
                </div>
              </div>
            </div>
          )}

          {/* Status messages */}
          {isLatest && (
            <div className="text-center text-sm text-blue-600 dark:text-blue-400 py-2">
              {t("update.alreadyLatest")}
            </div>
          )}

          {updateStatus === "checking" && !updateDownloading && (
            <div className="text-center text-sm text-gray-500 py-2">
              {t("update.checking")}...
            </div>
          )}

          {updateStatus === "error" && !updateDownloading && (
            <div className="text-center text-sm text-red-500 py-2">
              {t("update.checkFailed")}
            </div>
          )}

          {/* Release notes */}
          {updateInfo?.release_notes && (
            <div>
              <div className="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2 uppercase tracking-wide">
                {t("update.releaseNotes")}
              </div>
              <div className="text-sm text-gray-700 dark:text-gray-300 bg-gray-50 dark:bg-gray-900 rounded-md p-3 max-h-48 overflow-auto whitespace-pre-wrap font-mono text-xs leading-relaxed">
                {updateInfo.release_notes}
              </div>
            </div>
          )}

          {/* Download progress */}
          {updateDownloading && (
            <div>
              <div className="flex justify-between text-xs text-gray-500 mb-1">
                <span>{t("update.downloading")}</span>
                <span>{updateProgress}%</span>
              </div>
              <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                <div
                  className="bg-green-500 h-2 rounded-full transition-all duration-300"
                  style={{ width: `${updateProgress}%` }}
                />
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-5 py-4 border-t border-gray-200 dark:border-gray-700">
          <button
            onClick={() => {
              if (updateInfo?.release_notes) {
                // Open the release page — user needs to configure owner/repo
                const config = useNodeStore.getState().config;
                if (config?.githubOwner && config?.githubRepo && updateInfo) {
                  const url = `https://github.com/${config.githubOwner}/${config.githubRepo}/releases/tag/v${updateInfo.latest_version}`;
                  import("@tauri-apps/plugin-shell").then((shell) => {
                    shell.open(url);
                  });
                }
              }
            }}
            className="flex items-center gap-1 text-xs text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 transition-colors"
          >
            <ExternalLink size={12} />
            {t("update.viewOnGitHub")}
          </button>

          {hasUpdate && (
            <button
              onClick={handleDownloadUpdate}
              disabled={updateDownloading}
              className="flex items-center gap-2 px-4 py-2 bg-green-600 hover:bg-green-700 disabled:bg-gray-400 text-white text-sm font-medium rounded-md transition-colors"
            >
              <Download size={14} />
              {updateDownloading ? t("update.downloading") : t("update.updateNow")}
            </button>
          )}

          {!hasUpdate && (
            <button
              onClick={() => setShowUpdateModal(false)}
              className="px-4 py-2 bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 text-sm font-medium rounded-md transition-colors"
            >
              {t("common.close")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
