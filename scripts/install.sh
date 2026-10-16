#!/bin/bash
# Install the Sam macOS window.
#
#   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/install.sh)"
#
# Downloads the prebuilt binary from the latest GitHub release and verifies it
# against checksums.txt. Falls back to building from source when the release has
# no binary for this Mac.

# macOS /bin/bash is 3.2. Keep this script compatible with that.

set -eu
set -o pipefail

BIN_NAME="sam"
DEST_NAME="sam"
CARGO_FEATURE="gui"
LABEL="Sam GUI"
APP_BUNDLE="Sam.app"

GITHUB_REPO="luisitoluis/sam-macos-rs"
REPO_URL="https://github.com/$GITHUB_REPO.git"
BRANCH="master"
PREFIX="${SAM_PREFIX:-$HOME/.sam}"
SRC="$PREFIX/src"
BIN_DIR="${SAM_BIN_DIR:-$HOME/.local/bin}"
APP_DIR="${SAM_APP_DIR:-$HOME/Applications}"
METHOD="${SAM_METHOD:-auto}"
VERSION="${SAM_VERSION:-}"
INSECURE="${SAM_INSECURE:-0}"

ohai() {
  printf "\033[1;34m==>\033[0m \033[1m%s\033[0m\n" "$*"
}

warn() {
  printf "\033[1;33mWarning:\033[0m %s\n" "$*" >&2
}

abort() {
  printf "\033[1;31mError:\033[0m %s\n" "$*" >&2
  exit 1
}

have() {
  command -v "$1" >/dev/null 2>&1
}

usage() {
  cat <<EOF
Install the $LABEL ($DEST_NAME).

Options:
  --method METHOD   auto (default), binary or source
  --version TAG     Install a specific release, e.g. v1.0.2 (default: latest)
  --dir DIR         Command directory (default: \$HOME/.local/bin)
  --app-dir DIR     Where the GUI's Sam.app goes (default: \$HOME/Applications)
  --insecure        Skip checksum verification (not recommended)
  -h, --help        Show this help

The same settings can be given as SAM_METHOD, SAM_VERSION, SAM_BIN_DIR,
SAM_APP_DIR and SAM_INSECURE=1 environment variables.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --method)
      [[ $# -ge 2 ]] || abort "--method needs a value."
      METHOD="$2"
      shift 2
      ;;
    --version)
      [[ $# -ge 2 ]] || abort "--version needs a value."
      VERSION="$2"
      shift 2
      ;;
    --dir)
      [[ $# -ge 2 ]] || abort "--dir needs a value."
      BIN_DIR="$2"
      shift 2
      ;;
    --app-dir)
      [[ $# -ge 2 ]] || abort "--app-dir needs a value."
      APP_DIR="$2"
      shift 2
      ;;
    --insecure)
      INSECURE=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      abort "Unknown option: $1 (see --help)"
      ;;
  esac
done

case "$METHOD" in
  auto | binary | source) ;;
  *) abort "Unknown method: $METHOD. Use auto, binary or source." ;;
esac

if [[ "${EUID:-$(id -u)}" -eq 0 ]]; then
  abort "Run this as your user, not root."
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  abort "The $LABEL install is macOS-only."
fi

tmpdir=""
cleanup() {
  if [[ -n "$tmpdir" ]]; then
    rm -rf "$tmpdir"
  fi
}
trap cleanup EXIT

detect_arch() {
  case "$(uname -m)" in
    arm64 | aarch64) echo "arm64" ;;
    x86_64 | amd64) echo "amd64" ;;
    *) abort "Unsupported architecture: $(uname -m)." ;;
  esac
}

resolve_tag() {
  if [[ -n "$VERSION" ]]; then
    case "$VERSION" in
      v*) echo "$VERSION" ;;
      *) echo "v$VERSION" ;;
    esac
    return 0
  fi
  curl -fsSL "https://api.github.com/repos/$GITHUB_REPO/releases/latest" 2>/dev/null \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    | head -n 1
}

# Returns 2 when the release has no binary for this Mac, so auto can fall back.
install_binary() {
  local arch tag version archive base
  arch="$(detect_arch)"
  tag="$(resolve_tag)"
  if [[ -z "$tag" ]]; then
    warn "Could not find the latest release of $GITHUB_REPO."
    return 2
  fi
  version="${tag#v}"
  archive="sam_${version}_darwin_${arch}.tar.gz"
  base="https://github.com/$GITHUB_REPO/releases/download/$tag"

  ohai "Downloading $archive ($tag)"
  tmpdir="$(mktemp -d)" || abort "Could not create a temporary directory."
  if ! curl -fsSL -o "$tmpdir/$archive" "$base/$archive"; then
    warn "Release $tag has no $archive."
    return 2
  fi

  if [[ "$INSECURE" == "1" ]]; then
    warn "Skipping checksum verification (--insecure)."
  else
    ohai "Verifying checksum"
    curl -fsSL -o "$tmpdir/checksums.txt" "$base/checksums.txt" \
      || abort "Could not download checksums.txt for $tag. Re-run with --insecure to skip verification."
    grep -E "[[:space:]]$archive\$" "$tmpdir/checksums.txt" >"$tmpdir/expected.txt" \
      || abort "checksums.txt has no entry for $archive."
    (cd "$tmpdir" && shasum -a 256 -c expected.txt >/dev/null) \
      || abort "Checksum mismatch for $archive. The download may be corrupted or tampered with."
  fi

  mkdir -p "$tmpdir/extract"
  tar -xzf "$tmpdir/$archive" -C "$tmpdir/extract" || abort "Could not extract $archive."
  install_payload "$tmpdir/extract"
}

# Installs from a directory holding $APP_BUNDLE (GUI) or $BIN_NAME (TUI).
install_payload() {
  local from="$1"
  mkdir -p "$BIN_DIR" || abort "Could not create $BIN_DIR."

  if [[ -z "$APP_BUNDLE" ]]; then
    [[ -f "$from/$BIN_NAME" ]] || abort "$BIN_NAME is missing from the download."
    ditto "$from/$BIN_NAME" "$BIN_DIR/$DEST_NAME" || abort "Could not copy $DEST_NAME to $BIN_DIR."
    chmod 755 "$BIN_DIR/$DEST_NAME"
    xattr -d com.apple.quarantine "$BIN_DIR/$DEST_NAME" 2>/dev/null || true
    return 0
  fi

  [[ -d "$from/$APP_BUNDLE" ]] || abort "$APP_BUNDLE is missing from the download."
  mkdir -p "$APP_DIR" || abort "Could not create $APP_DIR."
  rm -rf "${APP_DIR:?}/$APP_BUNDLE"
  ditto "$from/$APP_BUNDLE" "$APP_DIR/$APP_BUNDLE" || abort "Could not copy $APP_BUNDLE to $APP_DIR."
  xattr -dr com.apple.quarantine "$APP_DIR/$APP_BUNDLE" 2>/dev/null || true

  # The command opens the app through LaunchServices so no Terminal window stays attached.
  cat >"$BIN_DIR/$DEST_NAME" <<EOF
#!/bin/sh
exec open -a "$APP_DIR/$APP_BUNDLE" --args "\$@"
EOF
  chmod 755 "$BIN_DIR/$DEST_NAME"
}

install_from_source() {
  ohai "Building the $LABEL from source"
  printf "Source:  %s (%s)\n" "$REPO_URL" "$BRANCH"
  printf "Build:   %s\n" "$SRC"

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
    # shellcheck disable=SC1091
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

  ohai "Building $BIN_NAME"
  (
    cd "$SRC"
    cargo build --locked --release --bin "$BIN_NAME" --no-default-features --features "$CARGO_FEATURE"
  ) || abort "Build failed."

  local target_dir binary
  target_dir="${CARGO_TARGET_DIR:-$SRC/target}"
  binary="$target_dir/release/$BIN_NAME"
  [[ -f "$binary" ]] || abort "Build finished but $binary is missing."

  if [[ -z "$APP_BUNDLE" ]]; then
    install_payload "$target_dir/release"
    return 0
  fi

  if [[ -z "$tmpdir" ]]; then
    tmpdir="$(mktemp -d)" || abort "Could not create a temporary directory."
  fi
  ohai "Packaging $APP_BUNDLE"
  /bin/bash "$SRC/scripts/make-app.sh" "$binary" "$tmpdir" >/dev/null || abort "Could not create $APP_BUNDLE."
  install_payload "$tmpdir"
}

add_to_path() {
  case ":$PATH:" in
    *":$BIN_DIR:"*) return 0 ;;
  esac
  local profile
  case "${SHELL:-}" in
    */bash) profile="$HOME/.bash_profile" ;;
    */zsh) profile="$HOME/.zprofile" ;;
    *) profile="$HOME/.profile" ;;
  esac
  if [[ ! -f "$profile" ]] || ! grep -F -q "$BIN_DIR" "$profile"; then
    printf '\n# %s\nexport PATH="%s:$PATH"\n' "$LABEL" "$BIN_DIR" >>"$profile"
    ohai "Added $BIN_DIR to PATH in $profile"
  fi
}

ohai "This will install the $LABEL"
if [[ -n "$APP_BUNDLE" ]]; then
  printf "App:     %s/%s\n" "$APP_DIR" "$APP_BUNDLE"
fi
printf "Command: %s/%s\n" "$BIN_DIR" "$DEST_NAME"

case "$METHOD" in
  source)
    install_from_source
    ;;
  binary)
    install_binary || abort "No prebuilt binary available. Run with --method source to build it."
    ;;
  auto)
    if ! install_binary; then
      ohai "Falling back to building from source"
      install_from_source
    fi
    ;;
esac

add_to_path

ohai "$LABEL installed"
if [[ -n "$APP_BUNDLE" ]]; then
  printf "Open Sam from Launchpad, Spotlight or %s, or run: %s\n" "$APP_DIR" "$DEST_NAME"
else
  printf "Open a new terminal and run: %s\n" "$DEST_NAME"
fi
printf "Steam needs to have been launched once before the first run.\n"
