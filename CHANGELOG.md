# Changelog

All notable changes to this project will be documented in this file.

## v1.1.3

- **Fixed PATH detection failing outright on a CJK Windows.** PATH is read through `powershell` and parsed as strict UTF-8, but PowerShell writes a redirected stdout in the console codepage (CP936/CP932) — so any non-ASCII path, such as a Chinese or Japanese user name, came back as bytes `serde_json` rejected with `invalid unicode code point`. The script now escapes every non-ASCII character as `\uXXXX`, making the output pure ASCII, which is byte-identical in every codepage.

## v1.1.2

- **PATH is now configured automatically.** The only way to write it was a button in the wizard or the warning strip, which was easy to miss — installs ended up with `node` still unavailable in a terminal. It is now written without asking at startup (once a version is active, so `nodeRoot` is settled) and after every successful switch.
- Removed the wizard's "environment variables" step — there is nothing left to configure by hand, and a manual step only repeated the problem it was meant to solve.
- A write that actually changes PATH now says so, so the "close and reopen your terminal" hint appears only when it applies.

## v1.1.1

- **Fixed the app freezing ("未响应") during auto-update.** The download ran on the main thread and blocked the webview for its whole duration; it now runs on a blocking thread pool.
- **Fixed the update progress bar stalling at 90%.** It was a fake timer capped at 90, unrelated to the download. The download loop now reports real percentages, matching how Node version downloads already work.
- Fixed a 1–2 second UI freeze when checking or configuring PATH — both commands spawn PowerShell and now run off the main thread.
- Added logging for PATH checks and writes, so a misconfigured machine can be diagnosed from the log.

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
