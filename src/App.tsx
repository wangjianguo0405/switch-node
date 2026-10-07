import { useEffect, useRef } from "react";
import { useNodeStore } from "./stores/useNodeStore";
import { TitleBar } from "./components/TitleBar";
import { Sidebar } from "./components/Sidebar";
import { VersionDetail } from "./components/VersionDetail";
import { StatusBar } from "./components/StatusBar";
import { PathWarning } from "./components/PathWarning";
import { SettingsDialog } from "./components/SettingsDialog";
import { SetupWizard } from "./components/SetupWizard";
import { DownloadProgress } from "./components/DownloadProgress";
import { UpdateModal } from "./components/UpdateModal";

function App() {
  const {
    initialize,
    wizardStep,
    settingsOpen,
    downloadProgress,
    loading,
    handleCheckUpdate,
  } = useNodeStore();
  const initialized = useRef(false);

  useEffect(() => {
    initialize();
  }, [initialize]);

  // Auto update check: first check 3s after init, then periodic
  useEffect(() => {
    if (!loading && !initialized.current) {
      initialized.current = true;
      const config = useNodeStore.getState().config;
      setTimeout(() => {
        handleCheckUpdate();
      }, 3000);
      if (config && config.updateIntervalMinutes > 0) {
        const timer = setInterval(() => {
          handleCheckUpdate();
        }, config.updateIntervalMinutes * 60 * 1000);
        return () => clearInterval(timer);
      }
    }
  }, [loading]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen bg-white dark:bg-gray-900">
        <div className="flex flex-col items-center gap-3">
          <div className="w-10 h-10 border-3 border-gray-200 border-t-green-500 rounded-full animate-spin" />
          <p className="text-gray-500 dark:text-gray-400 text-sm">
            Loading...
          </p>
        </div>
      </div>
    );
  }

  // First-run setup wizard
  if (wizardStep > 0) {
    return (
      <div className="h-screen flex flex-col bg-white dark:bg-gray-900">
        <SetupWizard />
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col bg-white dark:bg-gray-900 overflow-hidden">
      {/* Title Bar */}
      <TitleBar />

      {/* Main Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left Sidebar */}
        <div className="w-72 flex-shrink-0 border-r border-gray-200 dark:border-gray-700 overflow-hidden">
          <Sidebar />
        </div>

        {/* Right Detail Panel */}
        <div className="flex-1 overflow-hidden">
          <VersionDetail />
        </div>
      </div>

      {/* PATH not configured — node will not resolve in a terminal */}
      <PathWarning />

      {/* Status Bar */}
      <StatusBar />

      {/* Settings Modal */}
      {settingsOpen && <SettingsDialog />}

      {/* Update Modal */}
      <UpdateModal />

      {/* Download Progress */}
      {downloadProgress && <DownloadProgress />}
    </div>
  );
}

export default App;
