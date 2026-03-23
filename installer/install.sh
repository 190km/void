#!/bin/bash
# Void Terminal — cross-platform installer for macOS and Linux
# Usage: curl -fsSL https://void.sh/install.sh | bash

set -euo pipefail

APP_NAME="void"
REPO="190km/void"
INSTALL_DIR=""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}→${NC} $1"; }
ok()    { echo -e "${GREEN}✓${NC} $1"; }
warn()  { echo -e "${YELLOW}!${NC} $1"; }
error() { echo -e "${RED}✗${NC} $1" >&2; exit 1; }

# ── Detect platform ──────────────────────────────────────────────────
detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Darwin) OS="macos" ;;
        Linux)  OS="linux" ;;
        *)      error "Unsupported OS: $os" ;;
    esac

    case "$arch" in
        x86_64|amd64)   ARCH="x64" ;;
        aarch64|arm64)  ARCH="arm64" ;;
        *)              error "Unsupported architecture: $arch" ;;
    esac

    info "Detected: $OS $ARCH"
}

# ── Find latest release ─────────────────────────────────────────────
get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    local json

    if command -v curl &>/dev/null; then
        json=$(curl -fsSL "$url" 2>/dev/null) || error "Failed to fetch release info"
    elif command -v wget &>/dev/null; then
        json=$(wget -qO- "$url" 2>/dev/null) || error "Failed to fetch release info"
    else
        error "Neither curl nor wget found"
    fi

    VERSION=$(echo "$json" | grep '"tag_name"' | head -1 | sed 's/.*"v\?\([^"]*\)".*/\1/')
    [ -z "$VERSION" ] && error "Could not determine latest version"

    info "Latest version: v${VERSION}"
}

# ── Download ─────────────────────────────────────────────────────────
download() {
    local url="$1" dest="$2"
    info "Downloading $(basename "$dest")..."

    if command -v curl &>/dev/null; then
        curl -fsSL -o "$dest" "$url" || error "Download failed"
    else
        wget -qO "$dest" "$url" || error "Download failed"
    fi
}

# ── Install on macOS ─────────────────────────────────────────────────
install_macos() {
    local real_arch
    real_arch="$(uname -m)"  # x86_64 or arm64→aarch64
    [ "$real_arch" = "arm64" ] && real_arch="aarch64"
    local dmg_name="void-${VERSION}-${real_arch}-apple-darwin-setup.dmg"
    local url="https://github.com/${REPO}/releases/download/v${VERSION}/${dmg_name}"
    local tmp_dmg="/tmp/${dmg_name}"

    download "$url" "$tmp_dmg"

    info "Mounting DMG..."
    local mount_dir
    mount_dir=$(hdiutil attach -nobrowse -noautoopen "$tmp_dmg" 2>/dev/null | tail -1 | awk '{print $NF}')
    [ -z "$mount_dir" ] && error "Failed to mount DMG"

    local app_name
    app_name=$(ls "$mount_dir"/*.app 2>/dev/null | head -1)
    [ -z "$app_name" ] && { hdiutil detach "$mount_dir" -quiet; error "No .app found in DMG"; }

    INSTALL_DIR="/Applications"
    info "Installing to ${INSTALL_DIR}/$(basename "$app_name")..."
    rm -rf "${INSTALL_DIR}/$(basename "$app_name")"
    cp -R "$app_name" "$INSTALL_DIR/"

    hdiutil detach "$mount_dir" -quiet
    rm -f "$tmp_dmg"

    ok "Installed to ${INSTALL_DIR}/$(basename "$app_name")"
    echo ""
    info "Launch with: open '${INSTALL_DIR}/$(basename "$app_name")'"
}

# ── Install on Linux ─────────────────────────────────────────────────
install_linux() {
    # Prefer .deb on Debian/Ubuntu, otherwise tar.gz
    if command -v dpkg &>/dev/null && [ "$(uname -m)" = "x86_64" ]; then
        install_linux_deb
    else
        install_linux_tar
    fi
}

install_linux_deb() {
    local deb_name="void-${VERSION}-x86_64-linux-setup.deb"
    local url="https://github.com/${REPO}/releases/download/v${VERSION}/${deb_name}"
    local tmp_deb="/tmp/${deb_name}"

    download "$url" "$tmp_deb"

    info "Installing .deb package..."
    if [ "$(id -u)" -eq 0 ]; then
        dpkg -i "$tmp_deb" 2>/dev/null || apt-get install -f -y 2>/dev/null
    else
        sudo dpkg -i "$tmp_deb" 2>/dev/null || sudo apt-get install -f -y 2>/dev/null
    fi

    rm -f "$tmp_deb"
    INSTALL_DIR="/usr/bin"

    ok "Installed to ${INSTALL_DIR}/void"
    echo ""
    info "Launch with: void"
}

install_linux_tar() {
    local real_arch
    real_arch="$(uname -m)"
    [ "$real_arch" = "arm64" ] && real_arch="aarch64"
    local tar_name="void-${VERSION}-${real_arch}-linux-setup.tar.gz"
    local url="https://github.com/${REPO}/releases/download/v${VERSION}/${tar_name}"
    local tmp_tar="/tmp/${tar_name}"

    download "$url" "$tmp_tar"

    # Install to ~/.local/bin (user) or /usr/local/bin (root)
    if [ "$(id -u)" -eq 0 ]; then
        INSTALL_DIR="/usr/local/bin"
    else
        INSTALL_DIR="$HOME/.local/bin"
        mkdir -p "$INSTALL_DIR"
    fi

    info "Extracting to ${INSTALL_DIR}..."
    local tmp_extract
    tmp_extract=$(mktemp -d)
    tar -xzf "$tmp_tar" -C "$tmp_extract"

    local binary
    binary=$(find "$tmp_extract" -name "void" -type f | head -1)
    [ -z "$binary" ] && { rm -rf "$tmp_extract" "$tmp_tar"; error "Binary not found in archive"; }

    cp "$binary" "${INSTALL_DIR}/void"
    chmod +x "${INSTALL_DIR}/void"

    rm -rf "$tmp_extract" "$tmp_tar"

    ok "Installed to ${INSTALL_DIR}/void"

    # Check if install dir is in PATH
    if ! echo "$PATH" | tr ':' '\n' | grep -q "^${INSTALL_DIR}$"; then
        warn "${INSTALL_DIR} is not in your PATH"
        echo "  Add it with: export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo "  Or add to your ~/.bashrc / ~/.zshrc"
    fi

    echo ""
    info "Launch with: void"
}

# ── Main ─────────────────────────────────────────────────────────────
main() {
    echo ""
    echo -e "${CYAN}  Void Terminal Installer${NC}"
    echo ""

    detect_platform
    get_latest_version

    case "$OS" in
        macos) install_macos ;;
        linux) install_linux ;;
    esac

    echo ""
    ok "Done!"
}

main "$@"
