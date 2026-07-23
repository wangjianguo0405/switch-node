# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Dev Commands

```bash
# Full app (Rust + frontend) dev mode
npm run tauri dev

# Production build
npm run tauri build

# Frontend only — hot-reload Vite server
npm run dev

# TypeScript type-check
npx tsc --noEmit

# Rust check only (no linking)
cargo check --manifest-path src-tauri/Cargo.toml

# Generate app icons from a 1024x1024+ PNG
npx tauri icon src-tauri/icons/source.png
```

## Architecture

Tauri 2.x desktop app: **Rust backend** (`src-tauri/`) + **React 19 / TypeScript frontend** (`src/`). Manages multiple portable (zip-based) Node.js versions side-by-side under a single `nodeRoot` directory, switching the active version via a directory junction.

### Version switching (key design decisions)

- Uses `cmd /c mklink /J` for the `current` link — directory **junctions** do NOT require admin on Windows, unlike symlinks (`mklink /D`).
- PATH is only modified **once** during the setup wizard (`update_system_path` in `node_manager.rs`). After `{nodeRoot}\current` is in the system PATH, day-to-day switches change only the junction target and never touch the registry.
- No fallback/shim mode exists — junction creation is expected to always succeed on NTFS.
- `npmMirror` config change triggers `npm config set registry <url>` automatically in `set_config`.

### Rust backend (`src-tauri/src/`)

| Module | Role |
|--------|------|
| `lib.rs` | Entry point, registers Tauri plugins and commands, holds `AppState { config: Mutex<AppConfig> }` |
| `commands.rs` | Tauri `#[command]` handlers — every frontend invoke maps here |
| `config.rs` | Config load/save in json/toml/ini (priority: json > toml > ini), portable mode (file next to exe) |
| `node_manager.rs` | Local version scan, junction create/remove, system PATH update, `is_admin()` check |
| `downloader.rs` | Stream download zip → SHA256 verify → extract (strips zip top-level dir) → verify `node -v` |
| `remote.rs` | Fetch `index.json` from Node.js mirror, classify versions (Current/LTS/Maintenance/EOL), cache |
| `error.rs` | `AppError` enum (thiserror) with `From` impls for io/json/toml/reqwest → `Serialize` for frontend |

### Frontend (`src/`)

- **State**: Zustand store (`useNodeStore.ts`) — single source of truth for config, versions, download progress, UI state.
- **Components**: `Sidebar.tsx` (version list), `VersionDetail.tsx` (detail panel), `StatusBar.tsx` (bottom bar with status dot + refresh), `SettingsDialog.tsx` (tabbed settings form), `SetupWizard.tsx` (first-run wizard), `TitleBar.tsx` (custom titlebar).
- **i18n**: `lib/i18n.ts` — locale JSON files in `locales/` (zh-CN, en, ja), `t(key, replacements)` function, language persisted in config.
- **Types**: `lib/types.ts` — `AppConfig`, `LocalVersion`, `RemoteVersion`, `DownloadProgress`, etc.
- **Styling**: Tailwind CSS 4 via `@tailwindcss/vite` plugin, `global.css`.

### Tauri commands (Rust → Frontend)

```
get_config / set_config(partial)
scan_local_versions / fetch_remote_versions(force)
download_version(version) → emits "download-progress" events
switch_version(version)    → emits "version-switched" / "switch-error"
get_active_version / remove_version(version)
get_app_info / detect_system_node / open_release_notes(version)
```

### Config file

Portable mode: `config.json` (or `.toml` / `.ini`) lives next to the exe. Auto-generated on first run. Key fields: `nodeRoot`, `symlinkName` (default `"current"`), `mirror`, `architecture`, `npmMirror`, `language`, `theme`.

### Auto-update (`src-tauri/src/updater.rs`)

Custom green/portable auto-update via GitHub Releases API (no Tauri updater plugin):
- `check_update` → fetches latest release, compares semver, returns `UpdateInfo`
- `download_and_install` → downloads asset to `%TEMP%/switch-node_update/`, spawns `updater.bat`, exits app
- Batch script waits for process exit → copies new exe → restarts
- Config stored in `switch-node-config.json` next to exe (separate from main config)
- Supports optional GitHub token for higher API rate limits
- `update_interval: 0` = check only at startup

### Two separate dependency systems

- **Frontend** (`package.json`): npm — React, Zustand, Tailwind, Tauri API packages, lucide-react
- **Backend** (`src-tauri/Cargo.toml`): Cargo — tauri 2, reqwest, zip, sha2, serde, tokio, semver, chrono, ureq

## Release

### Version bump checklist

When publishing a new release, update these 3 files to the same version (e.g. `1.0.1`):

| File | Field |
|------|-------|
| `src-tauri/Cargo.toml` | `version = "x.y.z"` |
| `src-tauri/tauri.conf.json` | `"version": "x.y.z"` |
| `package.json` | `"version": "x.y.z"` (then run `npm install` to update lockfile) |

Also update `CHANGELOG.md` with the new version entry.

### Tag and push

```bash
git tag -a vx.y.z -m "vx.y.z: <summary>"
git push origin main --tags
```

The Release workflow builds `switch-node.exe` (portable, ~18MB) and creates a draft release on GitHub. Review and publish from the Releases page.
