#!/usr/bin/env bash

# Prevent script from hiding errors inside pipelines
set -o pipefail

# Configuration
INSTALL_DIR="/opt/infra"
LOCAL_BIN_DIR="$HOME/.local/bin"
BINARY_NAME="infra"

# Arch Linux style components (pacman/makepkg)
NC='\033[0m'
BOLD='\033[1m'
GREEN='\033[1;32m'
RED='\033[1;31m'
BLUE='\033[1;34m'
PURPLE='\033[1;35m'
GRAY='\033[0;90m'

# 1. Verify Cargo toolchain availability
if ! command -v cargo &> /dev/null; then
    echo -e " ${RED}->${NC} ${BOLD}Error:${NC} cargo (Rust toolchain) could not be found."
    echo -e "   Please install Rust via https://rustup.rs/ first."
    exit 1
fi

# 2. Compile target binary
echo -e "${BLUE}==>${NC} ${BOLD}Building release binary via cargo...${NC}"
if cargo build --release; then
    echo -e " ${GREEN}->${NC} Compilation finished ${GREEN}successfully${NC}."
else
    echo -e " ${RED}->${NC} ${BOLD}Error:${NC} Cargo build sequence failed."
    exit 1
fi

# 3. Prepare target filesystem nodes
mkdir -p "$INSTALL_DIR"
mkdir -p "$LOCAL_BIN_DIR"

# 4. Deploy release payload
echo -e "${BLUE}==>${NC} ${BOLD}Deploying binary...${NC}"
cp target/release/infra "$INSTALL_DIR/$BINARY_NAME"

# 5. Handle symlink routing redirection
ln -sf "$INSTALL_DIR/$BINARY_NAME" "$LOCAL_BIN_DIR/$BINARY_NAME"

# Truncate raw absolute $HOME paths to structural ~ representation for logs
PRINT_SRC=$(echo "$INSTALL_DIR/$BINARY_NAME" | sed "s|$HOME|~|g")
PRINT_DST=$(echo "$LOCAL_BIN_DIR/$BINARY_NAME" | sed "s|$HOME|~|g")
echo -e " ${GREEN}->${NC} Symlink created: ${BLUE}$PRINT_SRC${NC} ${GREEN}->${NC} ${BLUE}$PRINT_DST${NC}"

# 6. Verify environment variable array metrics
echo -e "${BLUE}==>${NC} ${BOLD}Checking environment PATH...${NC}"

if [[ ":$PATH:" != *":$LOCAL_BIN_DIR:"* ]]; then
    echo -e " ${RED}:: Warning:${NC} ${BOLD}$LOCAL_BIN_DIR${NC} is not active in your current PATH."
    
    # Active shell configuration shell target detection
    CURRENT_SHELL=$(basename "$SHELL")
    RC_FILE="$HOME/.bashrc"
    [[ "$CURRENT_SHELL" == "zsh" ]] && RC_FILE="$HOME/.zshrc"
    
    echo -e "     To inject it permanently, append the following line to your ${BOLD}$RC_FILE${NC}:"
    echo -e "     ${GRAY}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
else
    echo -e " ${GREEN}->${NC} Environment PATH is ${GREEN}OK${NC}. Directory is already registered."
fi

echo -e "${BLUE}==>${NC} ${GREEN}${BOLD}Installation complete!${NC}"
