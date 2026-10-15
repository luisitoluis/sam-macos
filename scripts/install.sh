#!/bin/bash
# Install the Sam macOS window.
#
#   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/install.sh)"

# macOS /bin/bash is 3.2. Keep this script compatible with that.

set -eu
set -o pipefail

REPO_URL="https://github.com/luisitoluis/sam-macos-rs.git"
BRANCH="master"
PREFIX="${SAM_PREFIX:-$HOME/.sam}"
SRC="$PREFIX/src"
BIN_DIR="${SAM_BIN_DIR:-$HOME/.local/bin}"

ohai() {
  printf "\033[1;34m==>\033[0m \033[1m%s\033[0m\n" "$*"
}

abort() {
  printf "\033[1;31mError:\033[0m %s\n" "$*" >&2
  exit 1
}

have() {
  command -v "$1" >/dev/null 2>&1
}

if [[ "${EUID:-$(id -u)}" -eq 0 ]]; then
  abort "Run this as your user, not root."
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  abort "The Sam window is macOS-only."
fi

ohai "This will install the Sam GUI"
printf "Source:  %s (%s)\n" "$REPO_URL" "$BRANCH"
printf "Build:   %s\n" "$SRC"
printf "Command: %s/sam\n" "$BIN_DIR"

if ! xcode-select -p >/dev/null 2>&1; then
  ohai "Requesting the Xcode Command Line Tools"
  xcode-select --install >/dev/null 2>&1 || true
  abort "Finish the Command Line Tools install, then run this script again."
fi

if ! have git; then
  abort "git was not found. Reinstall the Xcode Command Line Tools and run this script again."
fi

if ! have cargo; then
  ohai "Installing Rust"
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
    || abort "Rust install failed."
fi

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

have cargo || abort "cargo is not on PATH. Open a new terminal and run this script again."

mkdir -p "$PREFIX"

if [[ -d "$SRC/.git" ]]; then
  ohai "Updating $SRC"
  git -C "$SRC" fetch --depth 1 origin "$BRANCH" || abort "Could not update $SRC."
  git -C "$SRC" reset --hard FETCH_HEAD || abort "Could not update $SRC."
else
  if [[ -e "$SRC" ]]; then
    abort "$SRC exists and is not a git checkout. Move it aside and run this script again."
  fi
  ohai "Cloning $REPO_URL"
  git clone --depth 1 --branch "$BRANCH" "$REPO_URL" "$SRC" || abort "Clone failed."
fi

ohai "Building the GUI"
(
  cd "$SRC"
  cargo build --locked --release --bin Sam --no-default-features --features gui
) || abort "Build failed."

target_dir="${CARGO_TARGET_DIR:-$SRC/target}"
binary="$target_dir/release/Sam"
[[ -f "$binary" ]] || abort "Build finished but $binary is missing."

mkdir -p "$BIN_DIR"
ditto "$binary" "$BIN_DIR/sam"
chmod 755 "$BIN_DIR/sam"

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    case "${SHELL:-}" in
      */bash) profile="$HOME/.bash_profile" ;;
      */zsh) profile="$HOME/.zprofile" ;;
      *) profile="$HOME/.profile" ;;
    esac
    if [[ ! -f "$profile" ]] || ! grep -F -q "$BIN_DIR" "$profile"; then
      printf '\n# Sam GUI\nexport PATH="%s:$PATH"\n' "$BIN_DIR" >> "$profile"
      ohai "Added $BIN_DIR to PATH in $profile"
    fi
    ;;
esac

ohai "Sam GUI installed"
printf "Open a new terminal and run: sam\n"
printf "Steam needs to have been launched once before the first run.\n"
