# ProLaunch

A cross-platform desktop app that turns terminal commands into a visual
interface. Open any project folder — ProLaunch reads its `package.json`
scripts and shows them as buttons. Click to run, watch live logs, kill or
restart processes, see which ports are in use — no terminal needed.

Works on **macOS, Windows and Linux**. Built with Tauri + React + TypeScript.

## Features

- Create new projects from a template gallery (React, Vue, Svelte, Next.js,
  Nuxt, Angular, NestJS, Express, React Native and more — JS & TS).
- Open existing projects and run their scripts as buttons.
- Live logs, kill / restart, port monitoring.
- Search and pin commands; running commands float to the top.
- Recently opened projects, multi-tab workspace with browser-style shortcuts.

## Install

ProLaunch is **built from source on your own machine**. No code signing is
involved — a locally compiled binary is not flagged by macOS Gatekeeper or
Windows SmartScreen.

### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/vugarsafarzada/prolunch/main/install.sh | bash
```

The script:

1. Clones this repository into a temporary folder.
2. Installs Rust automatically (via `rustup`) if it is missing.
3. Installs the platform build dependencies.
4. Builds the app from source.
5. Installs it — macOS: `/Applications`, Linux: `~/.local/bin/prolaunch`.
6. Deletes the clone and all build artifacts.

**Prerequisites you must have already:** `git`, `curl`, and
[Node.js + npm](https://nodejs.org). On macOS the Xcode Command Line Tools are
installed on first run.

### Windows

Bash is not the native shell on Windows. Build manually:

1. Install [Node.js](https://nodejs.org), [Rust](https://rustup.rs), and the
   [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/).
2. Clone and build:

   ```powershell
   git clone --depth 1 https://github.com/vugarsafarzada/prolunch.git
   cd prolunch
   npm install
   npm run tauri build
   ```

3. The installer is created at
   `src-tauri/target/release/bundle/` — run the `.exe` / `.msi` from there.
4. Delete the cloned folder afterwards if you only need the installed app.

## Updating

Re-run the install command. It rebuilds the latest version from source.

## Development

```bash
npm install
npm run tauri dev
```
