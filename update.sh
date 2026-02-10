#!/usr/bin/env bash
# Update script for GitFetch - Because git pull is beneath you, obviously
# Rebuilds and reinstalls this magnificent waste of time

set -e

BOLD='\033[1m'
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BOLD}GitFetch Update${NC}"
echo "Updating the most over-engineered git wrapper known to mankind..."
echo

# Check if we're in a git repository
if [ ! -d ".git" ]; then
  echo -e "${RED}Error: Not in a git repository.${NC}"
  echo "Are you running this from the gitfetch directory? Brilliant."
  exit 1
fi

# Check if binary is currently installed
if ! command -v gitfetch &>/dev/null; then
  echo -e "${YELLOW}Warning: gitfetch not found in PATH.${NC}"
  echo "Have you even installed it yet? Run ./install.sh first, you muppet."
  exit 1
fi

# Show current version
CURRENT_VERSION=$(gitfetch --version 2>/dev/null || echo "unknown")
echo -e "Current version: ${BLUE}${CURRENT_VERSION}${NC}"
echo

# Stash any local changes (because you've probably been mucking about)
if ! git diff-index --quiet HEAD --; then
  echo -e "${YELLOW}Local changes detected. Stashing them...${NC}"
  git stash push -m "gitfetch-update-$(date +%s)"
  STASHED=true
else
  STASHED=false
fi

# Pull the latest changes
echo "Fetching latest changes from origin..."
git fetch origin

CURRENT_BRANCH=$(git branch --show-current)
echo "Current branch: $CURRENT_BRANCH"
echo

if [ -z "$CURRENT_BRANCH" ]; then
  echo -e "${RED}Error: Detached HEAD state detected.${NC}"
  echo "Sort yourself out before updating."
  exit 1
fi

echo "Pulling latest changes..."
if ! git pull origin "$CURRENT_BRANCH"; then
  echo -e "${RED}Git pull failed. Shocking, absolutely shocking.${NC}"
  if [ "$STASHED" = true ]; then
    echo "Your changes are still stashed. Run 'git stash pop' manually when you've sorted this mess out."
  fi
  exit 1
fi

# Check if there were actually any updates
if git diff --quiet HEAD@{1} HEAD; then
  echo -e "${GREEN}Already up to date. Well done for wasting everyone's time.${NC}"
  if [ "$STASHED" = true ]; then
    echo "Popping your stashed changes..."
    git stash pop
  fi
  exit 0
fi

echo -e "${GREEN}Updates pulled successfully!${NC}"
echo

# Check if Cargo.toml changed (dependencies might need updating)
if git diff --name-only HEAD@{1} HEAD | grep -q "Cargo.toml\|Cargo.lock"; then
  echo -e "${YELLOW}Dependencies changed. Running cargo update...${NC}"
  cargo update
fi

# Build the updated version
if ! command -v cargo &>/dev/null; then
  echo -e "${RED}Error: cargo not found.${NC}"
  echo "Did Rust magically uninstall itself? Remarkable."
  exit 1
fi

echo "Building updated release binary..."
if ! cargo build --release; then
  echo -e "${RED}Build failed. The new code is probably bollocks.${NC}"
  echo "Rolling back..."
  git reset --hard HEAD@{1}
  if [ "$STASHED" = true ]; then
    git stash pop
  fi
  exit 1
fi

echo -e "${GREEN}Build successful!${NC}"
echo

# Install the updated binary
INSTALL_DIR="/usr/local/bin"
BINARY_PATH="target/release/gitfetch"

if [ ! -f "$BINARY_PATH" ]; then
  echo -e "${RED}Binary not found at $BINARY_PATH${NC}"
  echo "The build lied to you. Fantastic."
  exit 1
fi

echo "Installing updated binary to $INSTALL_DIR..."
if [ -w "$INSTALL_DIR" ]; then
  cp "$BINARY_PATH" "$INSTALL_DIR/"
else
  echo "Need sudo privileges because you're not running as root (thankfully)..."
  sudo cp "$BINARY_PATH" "$INSTALL_DIR/"
fi

echo -e "${GREEN}Binary updated successfully!${NC}"
echo

# Check if completion scripts have changed
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SHELL_NAME=$(basename "$SHELL")

if git diff --name-only HEAD@{1} HEAD | grep -q "gitfetch\.\(bash\|zsh\|fish\)"; then
  echo -e "${YELLOW}Completion scripts have been updated.${NC}"

  case "$SHELL_NAME" in
  bash)
    if [ -f "/etc/bash_completion.d/gitfetch" ] || [ -f "$HOME/.local/share/bash-completion/completions/gitfetch" ]; then
      read -p "Reinstall bash completions? (y/n) " -n 1 -r
      echo
      if [[ $REPLY =~ ^[Yy]$ ]]; then
        if [ -f "/etc/bash_completion.d/gitfetch" ]; then
          echo "Updating system completions..."
          sudo cp "$SCRIPT_DIR/gitfetch.bash" "/etc/bash_completion.d/gitfetch"
          source /etc/bash_completion.d/gitfetch 2>/dev/null || true
        elif [ -f "$HOME/.local/share/bash-completion/completions/gitfetch" ]; then
          echo "Updating user completions..."
          cp "$SCRIPT_DIR/gitfetch.bash" "$HOME/.local/share/bash-completion/completions/gitfetch"
          source "$HOME/.local/share/bash-completion/completions/gitfetch" 2>/dev/null || true
        fi
        echo -e "${GREEN}Completions updated!${NC}"
      fi
    fi
    ;;

  zsh)
    if [ -f "/usr/local/share/zsh/site-functions/_gitfetch" ] || [ -f "$HOME/.zsh/completions/_gitfetch" ]; then
      read -p "Reinstall zsh completions? (y/n) " -n 1 -r
      echo
      if [[ $REPLY =~ ^[Yy]$ ]]; then
        if [ -f "/usr/local/share/zsh/site-functions/_gitfetch" ]; then
          echo "Updating system completions..."
          sudo cp "$SCRIPT_DIR/gitfetch.zsh" "/usr/local/share/zsh/site-functions/_gitfetch"
        elif [ -f "$HOME/.zsh/completions/_gitfetch" ]; then
          echo "Updating user completions..."
          cp "$SCRIPT_DIR/gitfetch.zsh" "$HOME/.zsh/completions/_gitfetch"
        fi
        echo -e "${GREEN}Completions updated! Run 'exec zsh' to reload.${NC}"
      fi
    fi
    ;;

  fish)
    if [ -f "$HOME/.config/fish/completions/gitfetch.fish" ]; then
      read -p "Reinstall fish completions? (y/n) " -n 1 -r
      echo
      if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "Updating fish completions..."
        cp "$SCRIPT_DIR/gitfetch.fish" "$HOME/.config/fish/completions/gitfetch.fish"
        echo -e "${GREEN}Completions updated! Run 'exec fish' to reload.${NC}"
      fi
    fi
    ;;
  esac
else
  echo "Completion scripts unchanged. How boring."
fi

# Pop stashed changes if we stashed any
if [ "$STASHED" = true ]; then
  echo
  echo "Restoring your local changes..."
  if ! git stash pop; then
    echo -e "${YELLOW}Warning: Couldn't automatically restore your changes.${NC}"
    echo "You'll need to manually resolve the conflicts. Well done."
  else
    echo -e "${GREEN}Local changes restored!${NC}"
  fi
fi

# Show new version
NEW_VERSION=$(gitfetch --version 2>/dev/null || echo "unknown")
echo
echo -e "${BOLD}Update Complete!${NC}"
echo -e "Previous version: ${BLUE}${CURRENT_VERSION}${NC}"
echo -e "Current version:  ${GREEN}${NEW_VERSION}${NC}"
echo
echo "Now bugger off and actually use the thing."
