#!/usr/bin/env bash

set -e
set -o pipefail

###############################################################################
# Project configuration
###############################################################################

PACKAGE_NAME="infra"
BINARY_NAME="$PACKAGE_NAME"
BUILD_ARGS=(
    "--release"
    # "--package" "$PACKAGE_NAME"
    # "--bin" "$PACKAGE_NAME"
    # "--features" "cli"
    # "--locked"
    # "--offline"
)

# Install prefix (optional)
INSTALL_PREFIX=""

###############################################################################
# Colors
###############################################################################

NC='\033[0m'
BOLD='\033[1m'
GREEN='\033[1;32m'
RED='\033[1;31m'
BLUE='\033[1;34m'
GRAY='\033[0;90m'

###############################################################################
# Platform detection
###############################################################################

OS="$(uname -s)"

case "$OS" in
    Linux)
        DEFAULT_PREFIX="/opt"
        BIN_DIR="$HOME/.local/bin"
        RC_FILE="$HOME/.bashrc"
        ;;

    Darwin)
        DEFAULT_PREFIX="/usr/local/lib"
        BIN_DIR="/usr/local/bin"
        RC_FILE="$HOME/.zshrc"
        ;;

    FreeBSD|OpenBSD|NetBSD|DragonFly)
        DEFAULT_PREFIX="/usr/local/lib"
        BIN_DIR="/usr/local/bin"
        RC_FILE="$HOME/.profile"
        ;;

    *)
        echo -e "${RED}Unsupported operating system: $OS${NC}"
        exit 1
        ;;
esac

INSTALL_PREFIX="${INSTALL_PREFIX:-$DEFAULT_PREFIX}"
INSTALL_DIR="$INSTALL_PREFIX/$PACKAGE_NAME"

###############################################################################
# Helpers
###############################################################################

echo_log() {
    echo -e "${BLUE}==>${NC} ${BOLD}$1${NC}"
}

echo_ok() {
    echo -e " ${GREEN}->${NC} $1"
}

echo_err() {
    echo -e " ${RED}->${NC} ${BOLD}$1${NC}"
}

pretty_path() {
    printf '%s' "${1/#$HOME/\~}"
}

###############################################################################
# Requirements
###############################################################################

command -v cargo >/dev/null || {
    err "Rust toolchain not found."
    echo "Install Rust from https://rustup.rs/"
    exit 1
}

###############################################################################
# Build
###############################################################################

echo_log "Running cargo build..."

cargo build "${BUILD_ARGS[@]}"

TARGET_DIR="target/debug"

for arg in "${BUILD_ARGS[@]}"; do
    if [[ "$arg" == "--release" ]]; then
        TARGET_DIR="target/release"
        break
    fi
done

SOURCE_BINARY="$TARGET_DIR/$PACKAGE_NAME"

[[ -f "$SOURCE_BINARY" ]] || {
    err "Compiled binary not found: $SOURCE_BINARY"
    exit 1
}

echo_ok "Compilation successful."

###############################################################################
# Install
###############################################################################

echo_log "Installing binaries..."

mkdir -p "$INSTALL_DIR"
mkdir -p "$BIN_DIR"

install -m755 "$SOURCE_BINARY" "$INSTALL_DIR/$BINARY_NAME"

ln -sf \
    "$INSTALL_DIR/$BINARY_NAME" \
    "$BIN_DIR/$BINARY_NAME"

echo_ok "Binary installed to ${BLUE}$(pretty_path "$INSTALL_DIR")${NC}"

###############################################################################
# PATH check
###############################################################################

echo_log "Registering in path..."

if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then

    echo
    echo -e "${RED}Warning:${NC} ${BOLD}$(pretty_path "$BIN_DIR")${NC} is not in PATH."
    echo
    echo "Add the following line to:"
    echo "  $RC_FILE"
    echo
    echo -e "${GRAY}export PATH=\"${BIN_DIR}:\$PATH\"${NC}"
    echo

else
    echo_ok "Symlink created on ${BLUE}$(pretty_path "$BIN_DIR/$BINARY_NAME")${NC}"
fi

###############################################################################
# Done
###############################################################################

echo
echo -e "${GREEN}${BOLD}Installation complete.${NC}"
