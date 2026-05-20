#!/usr/bin/env bash
#
# ProLaunch installer — clones the repo, builds the app from source on the
# user's machine, installs the result, then removes all build artifacts.
#
# Building locally means no code signing is needed: a locally compiled binary
# carries no macOS quarantine attribute and no Windows Mark-of-the-Web, so
# Gatekeeper / SmartScreen do not block it.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/vugarsafarzada/prolunch/main/install.sh | bash

set -euo pipefail

REPO_URL="https://github.com/vugarsafarzada/prolunch.git"
APP_NAME="prolaunch"
BUILD_DIR="$(mktemp -d)"

cleanup() { rm -rf "$BUILD_DIR"; }
trap cleanup EXIT

info() { printf '\033[1;34m==>\033[0m %s\n' "$1"; }
err()  { printf '\033[1;31mError:\033[0m %s\n' "$1" >&2; exit 1; }

OS="$(uname -s)"

# --- prerequisites -----------------------------------------------------------
command -v git  >/dev/null 2>&1 || err "git is required."
command -v curl >/dev/null 2>&1 || err "curl is required."
command -v npm  >/dev/null 2>&1 || err "Node.js + npm is required. Install from https://nodejs.org and re-run."

# Rust — installed automatically if missing.
if ! command -v cargo >/dev/null 2>&1; then
  info "Rust not found. Installing via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
command -v cargo >/dev/null 2>&1 || err "Rust installation failed."

# Platform build dependencies.
case "$OS" in
  Darwin)
    if ! xcode-select -p >/dev/null 2>&1; then
      info "Installing Xcode Command Line Tools (a dialog will open)..."
      xcode-select --install || true
      err "Finish the Xcode Command Line Tools install, then re-run this script."
    fi
    ;;
  Linux)
    if command -v apt-get >/dev/null 2>&1; then
      info "Installing Linux build dependencies (sudo required)..."
      sudo apt-get update
      sudo apt-get install -y \
        libwebkit2gtk-4.1-dev build-essential curl wget file \
        libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
    else
      info "Non-apt Linux detected. Ensure webkit2gtk and build tools are installed:"
      info "https://tauri.app/start/prerequisites/"
    fi
    ;;
  *)
    err "Unsupported OS: $OS. See the Windows instructions in the README."
    ;;
esac

# --- clone + build -----------------------------------------------------------
info "Cloning repository..."
git clone --depth 1 "$REPO_URL" "$BUILD_DIR/src"
cd "$BUILD_DIR/src"

info "Installing frontend dependencies..."
npm install

info "Building the app (this can take several minutes)..."
npm run tauri build

# --- install the built app ---------------------------------------------------
case "$OS" in
  Darwin)
    APP_BUNDLE="$(find src-tauri/target/release/bundle/macos -maxdepth 1 -name '*.app' 2>/dev/null | head -1)"
    [ -n "$APP_BUNDLE" ] || err "Build output not found."
    info "Installing to /Applications..."
    rm -rf "/Applications/$(basename "$APP_BUNDLE")"
    cp -R "$APP_BUNDLE" /Applications/
    info "Done. Launch ProLaunch from Applications or Spotlight."
    ;;
  Linux)
    if command -v apt-get >/dev/null 2>&1; then
      # .deb path — dpkg registers the menu entry, icon and PATH automatically.
      DEB="$(find src-tauri/target/release/bundle/deb -maxdepth 1 -name '*.deb' 2>/dev/null | head -1)"
      [ -n "$DEB" ] || err "Build output (.deb) not found."
      info "Installing the .deb package (sudo required)..."
      sudo apt-get install -y "$(realpath "$DEB")"
      info "Done. ProLaunch is now in your application menu."
    else
      # AppImage path — install the binary and write a desktop entry by hand.
      APPIMAGE="$(find src-tauri/target/release/bundle/appimage -maxdepth 1 -name '*.AppImage' 2>/dev/null | head -1)"
      [ -n "$APPIMAGE" ] || err "Build output not found."
      DEST="$HOME/.local/bin"
      ICON_DIR="$HOME/.local/share/icons"
      APPS_DIR="$HOME/.local/share/applications"
      mkdir -p "$DEST" "$ICON_DIR" "$APPS_DIR"
      cp "$APPIMAGE" "$DEST/$APP_NAME"
      chmod +x "$DEST/$APP_NAME"
      cp src-tauri/icons/128x128.png "$ICON_DIR/$APP_NAME.png"
      cat > "$APPS_DIR/$APP_NAME.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=ProLaunch
Comment=Run project scripts visually
Exec=$DEST/$APP_NAME
Icon=$ICON_DIR/$APP_NAME.png
Terminal=false
Categories=Development;Utility;
EOF
      command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS_DIR" || true
      info "Done. ProLaunch is now in your application menu."
    fi
    ;;
esac

info "Cleaning up build files..."
# BUILD_DIR (clone + target + node_modules) is removed by the EXIT trap.
info "Installation complete."
