import { Check, Download } from "lucide-react";

interface VersionItem {
  version: string;
  status: "installed" | "available";
  isActive: boolean;
  isCorrupted: boolean;
  lts: string | null;
  statusColor: string;
  statusLabel: string;
}

interface VersionListProps {
  versions: VersionItem[];
  selectedVersion: string | null;
  onSelect: (version: string) => void;
}

function StatusDot({ color }: { color: string }) {
  const colorMap: Record<string, string> = {
    current: "bg-node-green",
    maintenance: "bg-lts-maintenance",
    eol: "bg-eol",
  };
  return (
    <div
      className={`w-2 h-2 rounded-full flex-shrink-0 ${colorMap[color] || "bg-gray-400"}`}
    />
  );
}

function StatusTag({ color, label, lts }: { color: string; label: string; lts: string | null }) {
  const colorClasses: Record<string, string> = {
    current: "bg-node-green text-white",
    maintenance: "bg-lts-maintenance text-white",
    eol: "bg-eol text-white",
  };

  // For local (installed) versions without remote status, don't show a tag
  if (!label) return null;

  return (
    <span
      className={`text-[10px] px-1.5 py-0.5 rounded font-medium flex-shrink-0 ${
        colorClasses[color] || "bg-gray-400 text-white"
      }`}
    >
      {label}
      {lts && label === "LTS" ? ` ${lts}` : ""}
    </span>
  );
}

export function VersionList({
  versions,
  selectedVersion,
  onSelect,
}: VersionListProps) {
  if (versions.length === 0) return null;

  return (
    <div className="space-y-0.5">
      {versions.map((v) => {
        const isSelected = selectedVersion === v.version;
        return (
          <button
            key={v.version}
            onClick={() => onSelect(v.version)}
            className={`w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-sm transition-colors text-left ${
              isSelected
                ? "bg-node-green/10 dark:bg-node-green/20 ring-1 ring-node-green/30"
                : "hover:bg-gray-100 dark:hover:bg-gray-700/50"
            } ${v.isCorrupted ? "opacity-60" : ""}`}
          >
            <StatusDot color={v.statusColor} />
            <span
              className={`flex-1 truncate font-mono text-xs ${
                v.isActive
                  ? "font-bold text-node-green dark:text-node-green-dark"
                  : "text-gray-700 dark:text-gray-300"
              }`}
            >
              {v.version}
            </span>
            <StatusTag color={v.statusColor} label={v.statusLabel} lts={v.lts} />
            {v.isActive && (
              <Check size={14} className="text-node-green flex-shrink-0" />
            )}
            {v.status === "available" && !v.isActive && (
              <Download size={12} className="text-gray-400 flex-shrink-0" />
            )}
            {v.isCorrupted && (
              <span className="text-[10px] text-red-500 flex-shrink-0">
                ⚠
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}
