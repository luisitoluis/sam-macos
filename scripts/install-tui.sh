#!/bin/bash
# Install the Sam terminal UI.
#
#   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/install-tui.sh)"

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
  abort "The Sam terminal UI install is macOS-only."
fi

ohai "This will install the Sam terminal UI"
printf "Source:  %s (%s)\n" "$REPO_URL" "$BRANCH"
printf "Build:   %s\n" "$SRC"
printf "Command: %s/sam-tui\n" "$BIN_DIR"

if ! xcode-select -p >/dev/null 2>&1; then
  ohai "Requesting the Xcode Command Line Tools"
  xcode-select --install >/dev/null 2>&1 || true
  abort "Finish the Command Line Tools install, then run this script again."
fi

if [[ "$(uname -m)" == "arm64" && ! -f /Library/Apple/usr/libexec/oah/libRosettaRuntime ]]; then
  ohai "Installing Rosetta 2"
  /usr/sbin/softwareupdate --install-rosetta --agree-to-license \
    || abort "Rosetta 2 is required on Apple Silicon. Install it, then run this script again."
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

ohai "Building the terminal UI"
(
  cd "$SRC"
  cargo build --locked --release --bin sam-tui --no-default-features --features cli
) || abort "Build failed."

target_dir="${CARGO_TARGET_DIR:-$SRC/target}"
binary="$target_dir/release/sam-tui"
[[ -f "$binary" ]] || abort "Build finished but $binary is missing."

mkdir -p "$BIN_DIR"
ditto "$binary" "$BIN_DIR/sam-tui"
chmod 755 "$BIN_DIR/sam-tui"

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    case "${SHELL:-}" in
      */bash) profile="$HOME/.bash_profile" ;;
      */zsh) profile="$HOME/.zprofile" ;;
      *) profile="$HOME/.profile" ;;
    esac
    if [[ ! -f "$profile" ]] || ! grep -F -q "$BIN_DIR" "$profile"; then
      printf '\n# Sam terminal UI\nexport PATH="%s:$PATH"\n' "$BIN_DIR" >> "$profile"
      ohai "Added $BIN_DIR to PATH in $profile"
    fi
    ;;
esac

ohai "Sam terminal UI installed"
printf "Open a new terminal and run: sam-tui\n"
printf "sam-tui --version prints the installed version.\n"
printf "Steam needs to have been launched once before the first run.\n"
