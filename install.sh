#!/usr/bin/env bash
# Installation script for GitFetch - The Package Manager from Hell
# Now with ACTUAL shell configuration because apparently hand-holding is required

set -e

BOLD='\033[1m'
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BOLD}GitFetch Installation${NC}"
echo "Building the most over-engineered git wrapper you've ever seen..."
echo

# Build the bloody thing
if ! command -v cargo &>/dev/null; then
  echo -e "${RED}Error: cargo not found.${NC}"
  echo "Install Rust from https://rustup.rs/ you absolute muppet"
  exit 1
fi

echo "Building release binary..."
cargo build --release

if [ $? -ne 0 ]; then
  echo -e "${RED}Build failed. Shocking, absolutely shocking.${NC}"
  exit 1
fi

echo -e "${GREEN}Build successful!${NC}"
echo

# Install the binary
INSTALL_DIR="/usr/local/bin"
BINARY_PATH="target/release/gitfetch"

if [ ! -f "$BINARY_PATH" ]; then
  echo -e "${RED}Binary not found at $BINARY_PATH${NC}"
  exit 1
fi

echo "Installing binary to $INSTALL_DIR..."
if [ -w "$INSTALL_DIR" ]; then
  cp "$BINARY_PATH" "$INSTALL_DIR/"
else
  echo "Need sudo privileges because you're not running as root (thankfully)..."
  sudo cp "$BINARY_PATH" "$INSTALL_DIR/"
fi

echo -e "${GREEN}Binary installed successfully!${NC}"
echo

# Detect shell and offer completion installation
echo -e "${BOLD}Shell Completions${NC}"
echo "Would you like to install intelligent tab completions?"
echo

SHELL_NAME=$(basename "$SHELL")
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "$SHELL_NAME" in
bash)
  echo "Detected Bash shell"

  if [ ! -f "$SCRIPT_DIR/gitfetch.bash" ]; then
    echo -e "${RED}Error: gitfetch.bash not found in $SCRIPT_DIR${NC}"
    echo "Make sure you have all three completion scripts in the gitfetch directory."
    exit 1
  fi

  read -p "Install bash completions? (y/n) " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    COMPLETION_DIR="/etc/bash_completion.d"
    USER_COMPLETION_DIR="$HOME/.local/share/bash-completion/completions"

    # Try system-wide first
    if [ -d "$COMPLETION_DIR" ] && [ -w "$COMPLETION_DIR" ]; then
      echo "Installing to $COMPLETION_DIR/gitfetch..."
      cp "$SCRIPT_DIR/gitfetch.bash" "$COMPLETION_DIR/gitfetch"
      INSTALLED_PATH="$COMPLETION_DIR/gitfetch"
    elif [ -d "$COMPLETION_DIR" ]; then
      echo "Installing to $COMPLETION_DIR/gitfetch (requires sudo)..."
      sudo cp "$SCRIPT_DIR/gitfetch.bash" "$COMPLETION_DIR/gitfetch"
      INSTALLED_PATH="$COMPLETION_DIR/gitfetch"
    else
      # Fall back to user directory
      echo "System completion directory not available, installing to user directory..."
      mkdir -p "$USER_COMPLETION_DIR"
      cp "$SCRIPT_DIR/gitfetch.bash" "$USER_COMPLETION_DIR/gitfetch"
      INSTALLED_PATH="$USER_COMPLETION_DIR/gitfetch"

      # Make sure .bashrc sources completions
      if [ -f "$HOME/.bashrc" ]; then
        if ! grep -q "bash-completion/completions" "$HOME/.bashrc"; then
          echo "" >>"$HOME/.bashrc"
          echo "# Enable user completions" >>"$HOME/.bashrc"
          echo "if [ -d ~/.local/share/bash-completion/completions ]; then" >>"$HOME/.bashrc"
          echo "    for f in ~/.local/share/bash-completion/completions/*; do" >>"$HOME/.bashrc"
          echo "        [ -f \"\$f\" ] && source \"\$f\"" >>"$HOME/.bashrc"
          echo "    done" >>"$HOME/.bashrc"
          echo "fi" >>"$HOME/.bashrc"
          echo -e "${YELLOW}Added completion sourcing to ~/.bashrc${NC}"
        fi
      fi
    fi

    echo -e "${GREEN}Intelligent completions installed!${NC}"
    echo "Restart your shell or run: source $INSTALLED_PATH"
  fi
  ;;

zsh)
  echo "Detected Zsh shell"

  if [ ! -f "$SCRIPT_DIR/gitfetch.zsh" ]; then
    echo -e "${RED}Error: gitfetch.zsh not found in $SCRIPT_DIR${NC}"
    echo "Make sure you have all three completion scripts in the gitfetch directory."
    exit 1
  fi

  read -p "Install zsh completions? (y/n) " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    COMPLETION_DIR="/usr/local/share/zsh/site-functions"
    USER_COMPLETION_DIR="$HOME/.zsh/completions"

    # Try system-wide first
    if [ -d "$COMPLETION_DIR" ] && [ -w "$COMPLETION_DIR" ]; then
      echo "Installing to $COMPLETION_DIR/_gitfetch..."
      cp "$SCRIPT_DIR/gitfetch.zsh" "$COMPLETION_DIR/_gitfetch"
      INSTALLED_PATH="$COMPLETION_DIR/_gitfetch"
      NEEDS_FPATH=false
    elif [ -d "$COMPLETION_DIR" ]; then
      echo "Installing to $COMPLETION_DIR/_gitfetch (requires sudo)..."
      sudo cp "$SCRIPT_DIR/gitfetch.zsh" "$COMPLETION_DIR/_gitfetch"
      INSTALLED_PATH="$COMPLETION_DIR/_gitfetch"
      NEEDS_FPATH=false
    else
      # Fall back to user directory
      echo "System completion directory not available, installing to user directory..."
      mkdir -p "$USER_COMPLETION_DIR"
      cp "$SCRIPT_DIR/gitfetch.zsh" "$USER_COMPLETION_DIR/_gitfetch"
      INSTALLED_PATH="$USER_COMPLETION_DIR/_gitfetch"
      NEEDS_FPATH=true
    fi

    # Configure .zshrc if needed
    if [ "$NEEDS_FPATH" = true ] && [ -f "$HOME/.zshrc" ]; then
      # Check if fpath line already exists
      if ! grep -q "\.zsh/completions" "$HOME/.zshrc"; then
        # Find where to insert - before compinit, oh-my-zsh, prezto, etc.
        if grep -q "oh-my-zsh" "$HOME/.zshrc"; then
          # Oh My Zsh detected - add before it loads
          MARKER="source.*oh-my-zsh.sh"
          echo -e "${YELLOW}Oh My Zsh detected${NC}"
        elif grep -q "prezto" "$HOME/.zshrc"; then
          # Prezto detected
          MARKER="source.*prezto"
          echo -e "${YELLOW}Prezto detected${NC}"
        elif grep -q "compinit" "$HOME/.zshrc"; then
          # Plain compinit found
          MARKER="compinit"
          echo -e "${YELLOW}Compinit found${NC}"
        else
          # No framework detected, add at the end
          MARKER=""
          echo -e "${YELLOW}No completion framework detected${NC}"
        fi

        # Create backup
        cp "$HOME/.zshrc" "$HOME/.zshrc.backup.$(date +%s)"
        echo -e "${YELLOW}Created backup: ~/.zshrc.backup.$(date +%s)${NC}"

        if [ -n "$MARKER" ]; then
          # Insert before the marker
          sed -i.tmp "/^[^#]*$MARKER/i\\
# GitFetch completions\\
fpath=($USER_COMPLETION_DIR \$fpath)\\
" "$HOME/.zshrc"
          rm -f "$HOME/.zshrc.tmp"
          echo -e "${GREEN}Added fpath to .zshrc before framework initialization${NC}"
        else
          # Add at the end with compinit
          echo "" >>"$HOME/.zshrc"
          echo "# GitFetch completions" >>"$HOME/.zshrc"
          echo "fpath=($USER_COMPLETION_DIR \$fpath)" >>"$HOME/.zshrc"
          echo "autoload -Uz compinit && compinit" >>"$HOME/.zshrc"
          echo -e "${GREEN}Added fpath and compinit to .zshrc${NC}"
        fi
      else
        echo -e "${YELLOW}fpath already configured in .zshrc${NC}"
      fi
    fi

    echo -e "${GREEN}Intelligent completions installed!${NC}"
    echo "Restart your shell or run: exec zsh"
  fi
  ;;

fish)
  echo "Detected Fish shell"

  if [ ! -f "$SCRIPT_DIR/gitfetch.fish" ]; then
    echo -e "${RED}Error: gitfetch.fish not found in $SCRIPT_DIR${NC}"
    echo "Make sure you have all three completion scripts in the gitfetch directory."
    exit 1
  fi

  read -p "Install fish completions? (y/n) " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    COMPLETION_DIR="$HOME/.config/fish/completions"
    mkdir -p "$COMPLETION_DIR"
    echo "Installing to $COMPLETION_DIR/gitfetch.fish..."
    cp "$SCRIPT_DIR/gitfetch.fish" "$COMPLETION_DIR/gitfetch.fish"
    echo -e "${GREEN}Intelligent completions installed!${NC}"
    echo "Fish completions work immediately - just start a new shell session."
    echo "Run: exec fish"
  fi
  ;;

*)
  echo -e "${YELLOW}Shell '$SHELL_NAME' not automatically supported.${NC}"
  echo "Available completion scripts in this directory:"
  echo "  - gitfetch.bash (for Bash)"
  echo "  - gitfetch.zsh (for Zsh)"
  echo "  - gitfetch.fish (for Fish)"
  echo ""
  echo "Copy the appropriate one to your shell's completion directory manually."
  ;;
esac

echo
echo -e "${BOLD}Installation Complete!${NC}"
echo "Run 'gitfetch --help' to get started with this magnificent waste of effort."
echo
echo "Example commands:"
echo "  gitfetch -c user/repo       # Clone a repository"
echo "  gitfetch -s mangowc          # Search for repos by name"
echo "  gitfetch -l                  # List installed repos"
echo "  gitfetch -e                  # Easter egg (utterly pointless)"
