import { getCurrentWindow } from "@tauri-apps/api/window";
import { useNodeStore } from "../stores/useNodeStore";
import { t } from "../lib/i18n";
import { Settings, Minus, Square, X } from "lucide-react";

export function TitleBar() {
  const { setSettingsOpen } = useNodeStore();
  const appWindow = getCurrentWindow();

  return (
    <div className="flex items-center h-10 bg-gray-100 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 flex-shrink-0 select-none">
      {/* Left: Draggable area with app name */}
      <div data-tauri-drag-region className="flex items-center gap-2 pl-3 flex-1 h-full">
        <span className="text-yellow-500 text-sm">⚡</span>
        <span className="text-sm font-medium text-gray-700 dark:text-gray-200">
          {t("app.title")}
        </span>
      </div>

      {/* Right: Interactive buttons (outside drag region) */}
      <div className="flex items-center h-full">
        {/* Settings button */}
        <button
          className="h-full px-3 flex items-center justify-center hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors cursor-pointer"
          onClick={() => setSettingsOpen(true)}
          title={t("settings.title")}
        >
          <Settings size={16} className="text-gray-600 dark:text-gray-300" />
        </button>

        {/* Window controls */}
        <button
          className="h-full px-3 flex items-center justify-center hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors cursor-pointer"
          onClick={() => appWindow.minimize()}
        >
          <Minus size={16} className="text-gray-600 dark:text-gray-300" />
        </button>
        <button
          className="h-full px-3 flex items-center justify-center hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors cursor-pointer"
          onClick={() => appWindow.toggleMaximize()}
        >
          <Square size={14} className="text-gray-600 dark:text-gray-300" />
        </button>
        <button
          className="h-full px-3 flex items-center justify-center hover:bg-red-500 hover:text-white transition-colors cursor-pointer"
          onClick={() => appWindow.close()}
        >
          <X size={16} />
        </button>
      </div>
    </div>
  );
}
