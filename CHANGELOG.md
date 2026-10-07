# Changelog

All notable changes to this project will be documented in this file.

## v1.1.0

- **The app now configures PATH itself** — fixes `node` not being found in a terminal on a fresh machine. Writes the machine PATH first and falls back to the user PATH when the write is refused for lack of admin; writes nothing at all when the entry is already present.
- Added a fourth setup-wizard step for environment configuration, plus a persistent warning strip for when the wizard is skipped.
- Added a `relaunch_as_admin` command and a one-click "Restart as administrator" button, for when a system-installed Node.js outranks the managed one.
- Environment writes now go through the registry as `REG_EXPAND_SZ`, preserving `%SystemRoot%`-style references in PATH instead of freezing them into literals.
- Version metadata now loads on demand for installed patch versions that are not the latest of their major line.
- Removed the orphaned `elevate.ps1` script and the unused `is_admin()` probe.

## v1.0.1

- Pre-filled GitHub Owner/Repo defaults — auto-update works out of the box
- Async update check (reqwest) — no more UI freeze
- Fixed console window flashes at startup
- Moved download cache directory to Download settings tab
- Added About tab with manual update check
- Removed unnecessary `isAdmin` check
- Documentation updates and screenshot refresh

## v1.0.0

- Initial release
- Manage multiple portable Node.js versions side-by-side
- Switch active version via directory junction
- Download from official/taobao mirrors with SHA256 verification
- Setup wizard for first-run configuration
- i18n support: 简体中文, English, 日本語
- Auto-update via GitHub Releases (green/portable)
