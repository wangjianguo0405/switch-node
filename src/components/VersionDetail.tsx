import { useEffect, useState } from "react";
import { useNodeStore } from "../stores/useNodeStore";
import { t } from "../lib/i18n";
import { getVersionDetail, openReleaseNotes } from "../lib/commands";
import type { RemoteVersion } from "../lib/types";
import { ExternalLink, Download, RefreshCw, Trash2 } from "lucide-react";

export function VersionDetail() {
  const {
    localVersions,
    remoteVersions,
    selectedVersion,
    downloadProgress,
    switchVersion,
    downloadVersion,
    removeVersion,
  } = useNodeStore();

  // Find version data
  const local = localVersions.find((v) => v.version === selectedVersion);
  const remote = remoteVersions.find((v) => v.version === selectedVersion);

  // The remote list only carries each major line's latest patch, so an installed
  // older patch has no metadata here — fetch it on demand.
  const [fetchedDetail, setFetchedDetail] = useState<RemoteVersion | null>(null);
  const needsDetail = !!selectedVersion && !remote && !!local;

  useEffect(() => {
    setFetchedDetail(null);
    if (!needsDetail || !selectedVersion) return;

    let cancelled = false;
    getVersionDetail(selectedVersion)
      .then((detail) => {
        if (!cancelled) setFetchedDetail(detail);
      })
      .catch(() => {
        // Non-critical: panel just stays without the metadata table
      });
    return () => {
      cancelled = true;
    };
  }, [needsDetail, selectedVersion]);

  const meta =
    remote ?? (fetchedDetail?.version === selectedVersion ? fetchedDetail : null);

  if (!selectedVersion) {
    return (
      <div className="flex items-center justify-center h-full text-gray-400 dark:text-gray-500">
        <div className="text-center">
          <div className="text-4xl mb-3">⚡</div>
          <p className="text-sm">{t("detail.selectVersion")}</p>
        </div>
      </div>
    );
  }

  const versionData = local || meta;
  const isInstalled = !!local;
  const isActive = local?.isActive ?? false;
  const isCorrupted = local?.isCorrupted ?? false;
  const isDownloading =
    downloadProgress !== null &&
    downloadProgress.version === selectedVersion &&
    downloadProgress.stage !== "complete" &&
    downloadProgress.stage !== "error";

  if (!versionData) return null;

  const handleSwitch = async () => {
    try {
      await switchVersion(selectedVersion);
    } catch {
      // Error handled in store
    }
  };

  const handleDownload = async () => {
    try {
      await downloadVersion(selectedVersion);
    } catch {
      // Error handled in store
    }
  };

  const handleRemove = async () => {
    if (window.confirm(t("confirm.remove", { version: selectedVersion }))) {
      try {
        await removeVersion(selectedVersion);
      } catch {
        // Error handled in store
      }
    }
  };

  const handleOpenNotes = () => {
    openReleaseNotes(selectedVersion);
  };

  return (
    <div className="flex flex-col h-full p-6 overflow-y-auto">
      {/* Header */}
      <div className="flex items-center gap-3 mb-6">
        <div
          className={`w-3 h-3 rounded-full ${
            isActive ? "bg-node-green" : "bg-gray-300 dark:bg-gray-600"
          }`}
        />
        <h2 className="text-xl font-bold text-gray-800 dark:text-gray-100">
          {selectedVersion}
        </h2>
        {isActive && (
          <span className="text-xs px-2 py-0.5 rounded-full bg-node-green/10 text-node-green font-medium">
            {t("version.active")}
          </span>
        )}
        {meta?.isLatest && (
          <span className="text-xs px-2 py-0.5 rounded-full bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400 font-medium">
            Latest
          </span>
        )}
        {meta?.isLatestLts && (
          <span className="text-xs px-2 py-0.5 rounded-full bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400 font-medium">
            Latest LTS
          </span>
        )}
      </div>

      {/* Status badges */}
      {meta && (
        <div className="mb-4">
          <span
            className={`inline-block text-xs px-2.5 py-1 rounded-md font-medium text-white ${
              meta.statusColor === "current"
                ? "bg-node-green"
                : meta.statusColor === "maintenance"
                  ? "bg-lts-maintenance"
                  : "bg-eol"
            }`}
          >
            {meta.statusLabel}
          </span>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1.5">
            {meta.statusDescription}
          </p>
        </div>
      )}

      {/* Details table */}
      <div className="space-y-2 mb-6">
        {meta && (
          <>
            <DetailRow
              label={t("detail.versionNum")}
              value={meta.version}
            />
            <DetailRow
              label={t("detail.releaseDate")}
              value={meta.date}
            />
            <DetailRow
              label={t("detail.ltsCodename")}
              value={meta.lts || "-"}
            />
            <DetailRow label={t("detail.npmVersion")} value={meta.npm} />
            <DetailRow label={t("detail.v8Version")} value={meta.v8} />
            <DetailRow
              label={t("detail.opensslVersion")}
              value={meta.openssl}
            />
            <DetailRow
              label={t("detail.downloadSize")}
              value={`~${meta.downloadSizeMb} MB`}
            />
            <DetailRow
              label={t("detail.status")}
              value={meta.statusDescription}
            />
          </>
        )}
      </div>

      {/* Action Buttons */}
      <div className="flex flex-wrap gap-2">
        {isInstalled && !isActive && (
          <button
            onClick={handleSwitch}
            disabled={isCorrupted || isDownloading}
            className="flex items-center gap-1.5 px-4 py-2 bg-node-green hover:bg-node-green/90 text-white rounded-md text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <RefreshCw size={14} />
            {t("version.switch")}
          </button>
        )}

        {!isInstalled && (
          <button
            onClick={handleDownload}
            disabled={isDownloading}
            className="flex items-center gap-1.5 px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-md text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Download size={14} />
            {isDownloading ? t("download.downloading", { percent: "" }) : t("version.download")}
          </button>
        )}

        {isInstalled && isCorrupted && (
          <button
            onClick={handleDownload}
            disabled={isDownloading}
            className="flex items-center gap-1.5 px-4 py-2 bg-yellow-500 hover:bg-yellow-600 text-white rounded-md text-sm font-medium transition-colors"
          >
            <Download size={14} />
            {t("version.reinstall")}
          </button>
        )}

        {isInstalled && !isActive && (
          <button
            onClick={handleRemove}
            className="flex items-center gap-1.5 px-4 py-2 bg-red-500/10 hover:bg-red-500/20 text-red-600 dark:text-red-400 rounded-md text-sm font-medium transition-colors"
          >
            <Trash2 size={14} />
            {t("version.remove")}
          </button>
        )}

        {meta && (
          <button
            onClick={handleOpenNotes}
            className="flex items-center gap-1.5 px-4 py-2 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-md text-sm font-medium transition-colors"
          >
            <ExternalLink size={14} />
            {t("version.releaseNotes")}
          </button>
        )}
      </div>

      {/* Global packages reminder */}
      {isInstalled && (
        <div className="mt-4 p-3 rounded-md bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800">
          <p className="text-xs text-amber-700 dark:text-amber-300 leading-relaxed">
            {t("status.globalPackages")}
          </p>
        </div>
      )}
    </div>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex text-sm">
      <span className="w-28 text-gray-500 dark:text-gray-400 flex-shrink-0">
        {label}
      </span>
      <span className="text-gray-800 dark:text-gray-200 font-mono text-xs">
        {value}
      </span>
    </div>
  );
}
