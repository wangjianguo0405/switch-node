import { useNodeStore } from "../stores/useNodeStore";
import { t } from "../lib/i18n";
import { X, Check, AlertTriangle } from "lucide-react";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatSpeed(bps: number): string {
  if (bps < 1024) return `${bps.toFixed(0)} B/s`;
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
  return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
}

function formatETA(downloaded: number, total: number, speedBps: number): string {
  if (speedBps === 0 || total === 0) return "--";
  const remaining = total - downloaded;
  const seconds = Math.ceil(remaining / speedBps);
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
}

export function DownloadProgress() {
  const { downloadProgress, clearDownloadProgress } = useNodeStore();

  if (!downloadProgress) return null;

  const {
    version,
    stage,
    downloaded,
    total,
    speedBps,
  } = downloadProgress;

  const percent = total > 0 ? Math.round((downloaded / total) * 100) : 0;
  const isError = stage === "error";
  const isComplete = stage === "complete";

  return (
    <div className="fixed inset-0 bg-black/30 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-xl shadow-2xl p-6 w-[400px] max-w-[90vw]">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <h3 className="font-semibold text-gray-800 dark:text-gray-200">
            {isError
              ? t("download.error")
              : isComplete
                ? t("download.complete")
                : t("download.downloading", { percent: String(percent) })}
          </h3>
          <button
            onClick={clearDownloadProgress}
            className="p-1 rounded-md hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
          >
            <X size={16} className="text-gray-400" />
          </button>
        </div>

        {/* Version info */}
        <p className="text-sm text-gray-500 dark:text-gray-400 mb-3 font-mono">
          Node.js {version}
        </p>

        {/* Stage Indicator */}
        <div className="flex items-center justify-between mb-3">
          {(["connecting", "downloading", "verifying", "extracting", "installing"] as const).map(
            (s, i) => {
              const stageIndex = [
                "connecting",
                "downloading",
                "verifying",
                "extracting",
                "installing",
                "complete",
              ].indexOf(stage);
              const isDone = i < stageIndex;
              const isCurrent = i === stageIndex;

              return (
                <div key={s} className="flex flex-col items-center gap-1">
                  <div
                    className={`w-3 h-3 rounded-full transition-colors ${
                      isDone
                        ? "bg-node-green"
                        : isCurrent && !isError
                          ? "bg-blue-500 animate-pulse"
                          : "bg-gray-300 dark:bg-gray-600"
                    } ${isError ? "bg-red-500" : ""}`}
                  />
                </div>
              );
            }
          )}
        </div>
        <div className="flex justify-between text-[10px] text-gray-400 dark:text-gray-500 mb-4">
          <span>{t("download.connecting")}</span>
          <span>{t("download.verifying")}</span>
          <span>{t("download.installing")}</span>
        </div>

        {/* Progress bar */}
        {stage === "downloading" && (
          <>
            <div className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden mb-3">
              <div
                className="h-full bg-node-green rounded-full transition-all duration-300 ease-out"
                style={{ width: `${Math.min(percent, 100)}%` }}
              />
            </div>

            {/* Stats */}
            <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400">
              <span>
                {formatBytes(downloaded)} / {formatBytes(total)}
              </span>
              <span>
                {t("download.speed", { speed: formatSpeed(speedBps) })}
              </span>
              <span>
                {t("download.eta", {
                  time: formatETA(downloaded, total, speedBps),
                })}
              </span>
            </div>
          </>
        )}

        {/* Non-downloading stages */}
        {stage !== "downloading" && !isError && !isComplete && (
          <div className="flex items-center justify-center py-4">
            <div className="w-6 h-6 border-2 border-gray-300 border-t-node-green rounded-full animate-spin mr-3" />
            <span className="text-sm text-gray-500 dark:text-gray-400">
              {stage === "connecting" && t("download.connecting")}
              {stage === "verifying" && t("download.verifying")}
              {stage === "extracting" && t("download.extracting")}
              {stage === "installing" && t("download.installing")}
            </span>
          </div>
        )}

        {/* Error state */}
        {isError && (
          <div className="flex items-center gap-2 py-2 text-red-500">
            <AlertTriangle size={18} />
            <span className="text-sm">{t("download.error")}</span>
          </div>
        )}

        {/* Complete state */}
        {isComplete && (
          <div className="flex items-center gap-2 py-2 text-node-green">
            <Check size={18} />
            <span className="text-sm font-medium">
              {t("download.complete")}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
