import { useState } from "react";
import { useNodeStore } from "../stores/useNodeStore";
import { t } from "../lib/i18n";
import {
  FolderOpen,
  ChevronRight,
  ChevronLeft,
  CheckCircle2,
  Loader2,
} from "lucide-react";
import type { AppConfig } from "../lib/types";

export function SetupWizard() {
  const {
    config,
    wizardStep,
    remoteVersions,
    pathStatus,
    pathBusy,
    setWizardStep,
    updateConfig,
    downloadVersion,
    refreshRemote,
    refreshLocal,
    checkPath,
    configurePath,
    relaunchAsAdmin,
  } = useNodeStore();

  const [selectedDir, setSelectedDir] = useState(
    config?.nodeRoot || "D:\\Program Files\\nodejs"
  );
  const [selectedMirror, setSelectedMirror] = useState(
    config?.mirrorName || "official"
  );
  const [installing, setInstalling] = useState(false);
  const [installedVersion, setInstalledVersion] = useState<string | null>(null);
  const [pathError, setPathError] = useState<string | null>(null);

  // Get the latest LTS version for suggested install
  const latestLTS = remoteVersions.find((v) => v.isLatestLts);
  const suggestedVersion = latestLTS?.version || "v22.23.1";

  const handleBrowseDir = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        setSelectedDir(selected as string);
      }
    } catch {
      // Fallback: user can type manually
    }
  };

  const handleNext = async () => {
    if (wizardStep === 1) {
      // Save directory
      await updateConfig({
        nodeRoot: selectedDir,
      } as Partial<AppConfig>);
      setWizardStep(2);
    } else if (wizardStep === 2) {
      // Save mirror preference and proceed
      const mirrorUrl =
        selectedMirror === "taobao"
          ? "https://npmmirror.com/mirrors/node"
          : "https://nodejs.org/dist";
      await updateConfig({
        mirrorName: selectedMirror as AppConfig["mirrorName"],
        mirror: mirrorUrl,
      } as Partial<AppConfig>);
      setWizardStep(3);

      // Start installing latest LTS
      setInstalling(true);
      try {
        await refreshRemote(true);
        await downloadVersion(suggestedVersion);
        setInstalledVersion(suggestedVersion);
      } catch {
        // Error handled in store
      }
      setInstalling(false);
    } else if (wizardStep === 3) {
      // Move on to PATH setup
      setWizardStep(4);
      checkPath();
    }
  };

  const handleConfigurePath = async () => {
    setPathError(null);
    try {
      await configurePath();
    } catch (err) {
      setPathError(String(err));
    }
  };

  const handleSkip = () => {
    setWizardStep(0);
  };

  const handleFinish = async () => {
    await refreshLocal();
    setWizardStep(0);
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
        <h1 className="text-lg font-semibold text-gray-800 dark:text-gray-200">
          {t("wizard.title")}
        </h1>
        <button
          onClick={handleSkip}
          className="text-sm text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
        >
          {t("wizard.skip")}
        </button>
      </div>

      {/* Steps indicator */}
      <div className="flex items-center justify-center gap-2 px-6 py-4">
        {[1, 2, 3, 4].map((step) => (
          <div key={step} className="flex items-center gap-2">
            <div
              className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium transition-colors ${
                step <= wizardStep
                  ? "bg-node-green text-white"
                  : "bg-gray-200 dark:bg-gray-700 text-gray-500"
              }`}
            >
              {step}
            </div>
            {step < 4 && (
              <div
                className={`w-8 h-0.5 ${
                  step < wizardStep
                    ? "bg-node-green"
                    : "bg-gray-200 dark:bg-gray-700"
                }`}
              />
            )}
          </div>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 flex flex-col items-center justify-center px-6 pb-6">
        {wizardStep === 1 && (
          <div className="max-w-md w-full">
            <h2 className="text-lg font-medium text-gray-800 dark:text-gray-200 mb-2">
              {t("wizard.step1")}
            </h2>
            <p className="text-sm text-gray-500 dark:text-gray-400 mb-6">
              {t("wizard.step1Desc")}
            </p>
            <div className="flex gap-2">
              <input
                type="text"
                value={selectedDir}
                onChange={(e) => setSelectedDir(e.target.value)}
                className="flex-1 px-3 py-2 text-sm bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-1 focus:ring-node-green text-gray-800 dark:text-gray-200 font-mono"
                placeholder="D:\Program Files\nodejs"
              />
              <button
                onClick={handleBrowseDir}
                className="flex items-center gap-1.5 px-3 py-2 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-md text-sm transition-colors"
              >
                <FolderOpen size={14} />
                {t("settings.browse")}
              </button>
            </div>
          </div>
        )}

        {wizardStep === 2 && (
          <div className="max-w-md w-full">
            <h2 className="text-lg font-medium text-gray-800 dark:text-gray-200 mb-2">
              {t("wizard.step2")}
            </h2>
            <p className="text-sm text-gray-500 dark:text-gray-400 mb-6">
              {t("wizard.step2Desc")}
            </p>
            <div className="space-y-3">
              {[
                {
                  value: "official",
                  label: t("settings.official"),
                  desc: "https://nodejs.org/dist",
                },
                {
                  value: "taobao",
                  label: t("settings.taobao"),
                  desc: "https://npmmirror.com/mirrors/node",
                },
              ].map((opt) => (
                <label
                  key={opt.value}
                  className={`flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition-colors ${
                    selectedMirror === opt.value
                      ? "border-node-green bg-node-green/5"
                      : "border-gray-200 dark:border-gray-700 hover:border-gray-300"
                  }`}
                >
                  <input
                    type="radio"
                    name="mirror"
                    value={opt.value}
                    checked={selectedMirror === opt.value}
                    onChange={(e) => setSelectedMirror(e.target.value as typeof selectedMirror)}
                    className="mt-0.5 text-node-green focus:ring-node-green"
                  />
                  <div>
                    <div className="text-sm font-medium text-gray-800 dark:text-gray-200">
                      {opt.label}
                    </div>
                    <div className="text-xs text-gray-500 dark:text-gray-400">
                      {opt.desc}
                    </div>
                  </div>
                </label>
              ))}
            </div>
          </div>
        )}

        {wizardStep === 3 && (
          <div className="max-w-md w-full text-center">
            <h2 className="text-lg font-medium text-gray-800 dark:text-gray-200 mb-2">
              {t("wizard.step3")}
            </h2>
            <p className="text-sm text-gray-500 dark:text-gray-400 mb-6">
              {t("wizard.step3Desc")}
            </p>

            {installing && (
              <div className="flex flex-col items-center gap-3 py-8">
                <div className="w-12 h-12 border-4 border-gray-200 border-t-node-green rounded-full animate-spin" />
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  {t("wizard.installing", { version: suggestedVersion })}
                </p>
              </div>
            )}

            {installedVersion && (
              <div className="flex flex-col items-center gap-3 py-8">
                <div className="w-12 h-12 bg-green-100 dark:bg-green-900/30 rounded-full flex items-center justify-center">
                  <span className="text-2xl">✅</span>
                </div>
                <p className="text-sm text-gray-800 dark:text-gray-200 font-medium">
                  {installedVersion} 安装完成！
                </p>
              </div>
            )}
          </div>
        )}

        {wizardStep === 4 && (
          <div className="max-w-md w-full">
            <h2 className="text-lg font-medium text-gray-800 dark:text-gray-200 mb-2">
              {t("path.step4")}
            </h2>
            <p className="text-sm text-gray-500 dark:text-gray-400 mb-6">
              {t("path.step4Desc")}
            </p>

            {!pathStatus ? (
              <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                <Loader2 size={14} className="animate-spin" />
                {t("common.loading")}
              </div>
            ) : pathStatus.configured && !pathStatus.shadowedBy ? (
              <div className="flex items-center gap-2 p-3 rounded-lg bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-900">
                <CheckCircle2
                  size={16}
                  className="text-green-600 dark:text-green-500 flex-shrink-0"
                />
                <span className="text-sm text-green-800 dark:text-green-300">
                  {t("path.alreadyOk")}
                </span>
              </div>
            ) : (
              <div className="space-y-3">
                <p className="text-xs text-amber-700 dark:text-amber-400/90 break-all p-3 rounded-lg bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-900">
                  {pathStatus.shadowedBy
                    ? t("path.shadowedDesc", {
                        path: pathStatus.shadowedBy,
                        linkPath: pathStatus.linkPath,
                      })
                    : t("path.warningDesc", { linkPath: pathStatus.linkPath })}
                </p>

                {pathError && (
                  <p className="text-xs text-red-600 dark:text-red-400 break-all">
                    {t("path.adminRequired")} {pathError}
                  </p>
                )}

                <div className="flex gap-2">
                  <button
                    onClick={handleConfigurePath}
                    disabled={pathBusy}
                    className="flex items-center gap-1.5 px-4 py-2 bg-node-green hover:bg-node-green/90 text-white rounded-md text-sm font-medium transition-colors disabled:opacity-50"
                  >
                    {pathBusy && <Loader2 size={14} className="animate-spin" />}
                    {pathBusy ? t("path.configuring") : t("path.configure")}
                  </button>
                  {(pathError || pathStatus.shadowedBy) && (
                    <button
                      onClick={relaunchAsAdmin}
                      className="px-4 py-2 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-md text-sm transition-colors text-gray-800 dark:text-gray-200"
                    >
                      {t("path.relaunchAdmin")}
                    </button>
                  )}
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="flex justify-between px-6 py-4 border-t border-gray-200 dark:border-gray-700">
        <div>
          {wizardStep > 1 && (
            <button
              onClick={() => setWizardStep(wizardStep - 1)}
              className="flex items-center gap-1 px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md transition-colors"
            >
              <ChevronLeft size={14} />
              {t("wizard.back")}
            </button>
          )}
        </div>
        <div>
          {wizardStep < 4 && (
            <button
              onClick={handleNext}
              disabled={wizardStep === 3 && installing}
              className="flex items-center gap-1 px-6 py-2 bg-node-green hover:bg-node-green/90 text-white rounded-md text-sm font-medium transition-colors disabled:opacity-50"
            >
              {t("wizard.next")}
              <ChevronRight size={14} />
            </button>
          )}
          {wizardStep === 4 && (
            <button
              onClick={handleFinish}
              className="flex items-center gap-1 px-6 py-2 bg-node-green hover:bg-node-green/90 text-white rounded-md text-sm font-medium transition-colors"
            >
              {t("wizard.finish")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
