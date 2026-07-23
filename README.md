# Node.js Version Switcher

**switch-node** — A desktop app to download, install, and switch between portable Node.js versions.

[简体中文](README_zh-CN.md) | [日本語](README_ja.md)

## Features

- 📦 Manage multiple Node.js versions side-by-side (portable zip)
- 🔀 Instant switch via directory junction (`mklink /J`, no admin required)
- 📥 Built-in download with SHA256 verification
- 🌐 Mirror support (official / Taobao / custom)
- 🎨 LTS / Current / EOL classification with color labels
- 🌍 UI in English, Simplified Chinese, and Japanese

## Screenshots

![Main window](screenshots/main.png)

| Download in progress | Download complete | Settings |
|---|---|---|
| ![download](screenshots/download-in-progress.png) | ![complete](screenshots/download-complete.png) | ![settings](screenshots/setting.png) |

## Install

Download the latest `.msi` or portable `.zip` from [Releases](https://github.com/example/switch-node/releases).

## Usage

1. Launch the app (setup wizard on first run)
2. Select a Node.js version → click "Download"
3. After download, click "Switch"
4. Verify with `node -v` in terminal

### Settings

- **nodeRoot** — Where versions are stored (default: `D:\Program Files\nodejs`)
- **Mirror** — Download source (nodejs.org / npmmirror.com / custom)
- **npm Mirror** — Auto-sets npm registry via `npm config set registry`

## Development

```bash
# Install dependencies
npm install

# Dev mode (Rust + frontend)
npm run tauri dev

# Production build
npm run tauri build

# TypeScript type-check
npx tsc --noEmit

# Rust check
cargo check --manifest-path src-tauri/Cargo.toml
```

### Tech Stack

| Layer | Technology |
|---|---|
| Frontend | React 19, TypeScript, Tailwind CSS 4, Zustand 5, Vite 6 |
| Backend | Rust, Tauri 2.x, reqwest, zip, sha2, tokio |
| Desktop | Tauri 2.x |

## License

MIT
