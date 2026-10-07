import { useState } from "react";
import { useNodeStore } from "../stores/useNodeStore";
import { t } from "../lib/i18n";
import { AlertTriangle, Loader2 } from "lucide-react";

/// Persistent strip shown while `{nodeRoot}\current` is missing from PATH, or
/// present but outranked by a system Node.js — either way `node` in a terminal
/// resolves to the wrong thing (or to nothing at all).
export function PathWarning() {
  const { pathStatus, pathBusy, configurePath, relaunchAsAdmin } =
    useNodeStore();
  const [error, setError] = useState<string | null>(null);

  if (!pathStatus) return null;

  const shadowed = pathStatus.shadowedBy;
  const needsConfig = !pathStatus.configured;
  if (!needsConfig && !shadowed) return null;

  const handleConfigure = async () => {
    setError(null);
    try {
      await configurePath();
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div className="flex items-start gap-2 px-3 py-2 bg-amber-50 dark:bg-amber-950/40 border-t border-amber-200 dark:border-amber-900 text-xs flex-shrink-0">
      <AlertTriangle
        size={14}
        className="mt-0.5 flex-shrink-0 text-amber-600 dark:text-amber-500"
      />

      <div className="flex-1 min-w-0">
        <p className="font-medium text-amber-800 dark:text-amber-300">
          {needsConfig ? t("path.warningTitle") : t("path.shadowedTitle")}
        </p>
        <p className="text-amber-700 dark:text-amber-400/90 mt-0.5 break-all">
          {needsConfig
            ? t("path.warningDesc", { linkPath: pathStatus.linkPath })
            : t("path.shadowedDesc", {
                path: shadowed ?? "",
                linkPath: pathStatus.linkPath,
              })}
        </p>
        {needsConfig && shadowed && (
          <p className="text-amber-700 dark:text-amber-400/90 mt-0.5 break-all">
            {t("path.warningExtra", { path: shadowed })}
          </p>
        )}
        {error && (
          <p className="text-red-700 dark:text-red-400 mt-0.5 break-all">
            {t("path.adminRequired")} {error}
          </p>
        )}
      </div>

      <div className="flex items-center gap-2 flex-shrink-0">
        {needsConfig ? (
          <button
            onClick={handleConfigure}
            disabled={pathBusy}
            className="flex items-center gap-1 px-2.5 py-1 bg-amber-600 hover:bg-amber-700 text-white rounded transition-colors disabled:opacity-50"
          >
            {pathBusy && <Loader2 size={12} className="animate-spin" />}
            {pathBusy ? t("path.configuring") : t("path.configure")}
          </button>
        ) : (
          <button
            onClick={relaunchAsAdmin}
            className="px-2.5 py-1 bg-amber-600 hover:bg-amber-700 text-white rounded transition-colors"
          >
            {t("path.relaunchAdmin")}
          </button>
        )}
        {needsConfig && error && (
          <button
            onClick={relaunchAsAdmin}
            className="px-2.5 py-1 bg-gray-600 hover:bg-gray-700 text-white rounded transition-colors"
          >
            {t("path.relaunchAdmin")}
          </button>
        )}
      </div>
    </div>
  );
}
