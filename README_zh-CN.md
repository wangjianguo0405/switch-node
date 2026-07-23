# Node.js 版本切换器

**switch-node** — 一站式完成便携版 Node.js 下载、安装、切换的桌面应用。

[English](README.md) | [日本語](README_ja.md)

## 功能

- 📦 多版本 Node.js 统一管理（便携 zip 版）
- 🔀 通过目录联接（`mklink /J`）瞬时切换，无需管理员权限
- 📥 内置下载（SHA256 校验 + 解压）
- 🌐 支持镜像（官方 / 淘宝 / 自定义）
- 🎨 LTS / Current / EOL 分类与颜色标识
- 🌍 简体中文、英文、日文界面
- 🔄 内置自动更新（通过 GitHub Releases）

## 截图

![主界面](screenshots/main.png)

| 下载中 | 下载完成 | 设置 |
|---|---|---|
| ![下载中](screenshots/download-in-progress.png) | ![下载完成](screenshots/download-complete.png) | ![设置](screenshots/setting.png) |

## 安装

> ⚠️ **重要**：使用前请先卸载系统中已安装的 Node.js（控制面板 → 程序和功能），避免 PATH 冲突导致版本切换无效。

从 [Releases](https://github.com/wangjianguo0405/switch-node/releases) 下载最新的 `switch-node.exe`（绿色便携版，无需安装）。

## 使用

1. 启动应用（首次启动会打开设置向导）
2. 选择要下载的 Node.js 版本 → 点击"下载"
3. 下载完成后点击"切换"
4. 在终端运行 `node -v` 确认

### 设置

- **nodeRoot** — Node.js 安装目录（默认: `D:\Program Files\nodejs`）
- **镜像** — 下载源（nodejs.org / npmmirror.com / 自定义）
- **npm 镜像** — 自动执行 `npm config set registry`
- **自动更新** — 配置 GitHub Owner/Repo 后自动检查更新

## 开发

```bash
# 安装依赖
npm install

# 开发模式（Rust + 前端）
npm run tauri dev

# 生产构建
npm run tauri build

# TypeScript 类型检查
npx tsc --noEmit

# Rust 检查
cargo check --manifest-path src-tauri/Cargo.toml
```

### 技术栈

| 层 | 技术 |
|---|---|
| 前端 | React 19, TypeScript, Tailwind CSS 4, Zustand 5, Vite 6 |
| 后端 | Rust, Tauri 2.x, reqwest, zip, sha2, tokio |
| 桌面 | Tauri 2.x |

## 许可证

MIT
