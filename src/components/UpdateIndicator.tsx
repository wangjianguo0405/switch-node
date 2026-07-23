import { Loader2, Check, ArrowDown, AlertCircle, Settings } from "lucide-react";
import { useNodeStore } from "../stores/useNodeStore";
import { t } from "../lib/i18n";
import type { UpdateStatus } from "../lib/types";

export default function UpdateIndicator() {
  const updateStatus = useNodeStore((s) => s.updateStatus);
  const updateInfo = useNodeStore((s) => s.updateInfo);
  const setShowUpdateModal = useNodeStore((s) => s.setShowUpdateModal);
  const setSettingsOpen = useNodeStore((s) => s.setSettingsOpen);
  const handleCheckUpdate = useNodeStore((s) => s.handleCheckUpdate);

  const currentVersion = updateInfo?.current_version ?? "...";
  const latestVersion = updateInfo?.latest_version;

  const handleClick = async () => {
    if (updateStatus === "error" || updateStatus === "idle") {
      await handleCheckUpdate();
    }
    setShowUpdateModal(true);
  };

  return (
    <div className="flex items-center gap-1">
      <button
        className="flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 transition-colors disabled:opacity-50"
        disabled={updateStatus === "checking"}
        title={getTooltip(updateStatus, currentVersion, latestVersion)}
        onClick={handleClick}
      >
        <StatusIcon status={updateStatus} />
        {updateStatus !== "checking" && (
          <span className="font-mono">{`v${currentVersion}`}</span>
        )}
      </button>
      <button
        className="p-0.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
        title={t("update.settings")}
        onClick={(e) => {
          e.stopPropagation();
          setSettingsOpen(true);
        }}
      >
        <Settings size={11} />
      </button>
    </div>
  );
}

function getStatusColor(status: UpdateStatus): string {
  switch (status) {
    case "latest":
      return "text-blue-500";
    case "available":
      return "text-green-500";
    case "error":
      return "text-red-500";
    default:
      return "text-gray-400";
  }
}

function StatusIcon({ status }: { status: UpdateStatus }) {
  const color = getStatusColor(status);
  switch (status) {
    case "checking":
      return <Loader2 size={12} className="animate-spin text-gray-400" />;
    case "latest":
      return <Check size={13} className={color} strokeWidth={2.5} />;
    case "available":
      return <ArrowDown size={13} className={color} strokeWidth={2.5} />;
    case "error":
      return <AlertCircle size={12} className={color} />;
    default:
      return null;
  }
}

function getTooltip(
  status: UpdateStatus,
  currentVersion: string,
  latestVersion?: string
): string {
  switch (status) {
    case "idle":
      return `v${currentVersion}`;
    case "checking":
      return t("update.checking");
    case "latest":
      return t("update.latest", { version: currentVersion });
    case "available":
      return t("update.available", {
        version: latestVersion ?? "?",
      });
    case "error":
      return t("update.errorRetry");
  }
}
